//! Kerberos 5 parsing functions

use nom::{IResult,ErrorKind};
use der_parser::{parse_der,parse_der_bitstring,parse_der_generalstring,parse_der_integer,parse_der_generalizedtime,parse_der_octetstring,DerObject,DerObjectHeader,DerObjectContent,DerTag,DerError};
use std::str;

use krb5::*;

/// Parse a signed 32 bits integer
///
/// <pre>
/// Int32           ::= INTEGER (-2147483648..2147483647)
///                     -- signed values representable in 32 bits
/// </pre>
pub fn parse_der_int32(i:&[u8]) -> IResult<&[u8],i32> {
    map_res!(i, parse_der_integer,|x: DerObject| {
        match x.content {
            DerObjectContent::Integer(i) => {
                match i.len() {
                    1 => Ok(  i[0] as i8 as i32),
                    2 => Ok( (i[0] as i8 as i32) << 8  | (i[1] as i32) ),
                    3 => Ok( (i[0] as i8 as i32) << 16 | (i[1] as i32) << 8 | (i[2] as i32) ),
                    4 => Ok( (i[0] as i8 as i32) << 24 | (i[1] as i32) << 16 | (i[2] as i32) << 8 | (i[3] as i32) ),
                    _ => Err(DerError::IntegerTooLarge),
                }
            }
            _ => Err(DerError::DerTypeError)
        }
    })
}

/// Parse an unsigned 32 bits integer
///
/// <pre>
/// UInt32          ::= INTEGER (0..4294967295)
///                     -- unsigned 32 bit values
/// </pre>
pub fn parse_der_uint32(i:&[u8]) -> IResult<&[u8],u32> {
    map_res!(i, parse_der_integer,|x: DerObject| x.as_u32())
}

//  Microseconds    ::= INTEGER (0..999999)
//                      -- microseconds
fn parse_der_microseconds(i:&[u8]) -> IResult<&[u8],u32> {
    map_res!(i, parse_der_integer,|x: DerObject|
             x.as_u32().and_then(|x| if x <= 999999 {Ok(x)} else {Err(DerError::IntegerTooLarge)}))
}

/// Parse a Kerberos string object
///
/// <pre>
/// KerberosString  ::= GeneralString (IA5String)
/// </pre>
pub fn parse_kerberos_string(i: &[u8]) -> IResult<&[u8],String> {
    map_res!(
        i,
        parse_der_generalstring,
        |x: DerObject| {
            match str::from_utf8(x.as_slice().unwrap()) {
                Ok(r)  => Ok(r.to_owned()),
                Err(_) => Err("not a valid UTF-8 string")
            }
        }
    )
}

fn parse_kerberos_string_sequence(i: &[u8]) -> IResult<&[u8],Vec<String>> {
    map_res!(
        i,
        parse_der_struct!(
            v: many0!(parse_kerberos_string) >>
            ( v )
        ),
        |(hdr,t) : (DerObjectHeader,Vec<String>)| {
            if hdr.tag != DerTag::Sequence as u8 { return Err("not a sequence!"); }
            Ok(t)
        }
    )
}

/// Parse Kerberos flags
///
/// <pre>
/// KerberosFlags   ::= BIT STRING (SIZE (32..MAX))
///                     -- minimum number of bits shall be sent,
///                     -- but no fewer than 32
/// </pre>
pub fn parse_kerberos_flags(i: &[u8]) -> IResult<&[u8],DerObject> {
    parse_der_bitstring(i)
}

/// Parse of a Kerberos Realm
///
/// <pre>
/// Realm           ::= KerberosString
/// </pre>
pub fn parse_krb5_realm(i: &[u8]) -> IResult<&[u8],Realm> {
    map!(i, parse_kerberos_string, |s| Realm(s))
}

/// Parse Kerberos PrincipalName
///
/// <pre>
/// PrincipalName   ::= SEQUENCE {
///         name-type       [0] Int32,
///         name-string     [1] SEQUENCE OF KerberosString
/// }
/// </pre>
pub fn parse_krb5_principalname(i: &[u8]) -> IResult<&[u8],PrincipalName> {
    map_res!(
        i,
        parse_der_struct!(
            t: parse_der_tagged!(EXPLICIT 0, parse_der_int32) >>
            s: parse_der_tagged!(EXPLICIT 1, parse_kerberos_string_sequence) >>
            ( PrincipalName{
                name_type: NameType(t),
                name_string: s,
            })
        ),
        |(hdr,t) : (DerObjectHeader,PrincipalName)| {
            if hdr.tag != DerTag::Sequence as u8 { return Err("not a sequence!"); }
            Ok(t)
        }
    )
}

/// Parse of a Kerberos Time
///
/// <pre>
/// KerberosTime    ::= GeneralizedTime -- with no fractional seconds
/// </pre>
pub fn parse_kerberos_time(i: &[u8]) -> IResult<&[u8],DerObject> {
    parse_der_generalizedtime(i)
}

/// Parse Kerberos HostAddress
///
/// <pre>
/// HostAddress     ::= SEQUENCE  {
///         addr-type       [0] Int32,
///         address         [1] OCTET STRING
/// }
/// </pre>
pub fn parse_krb5_hostaddress<'a>(i: &'a[u8]) -> IResult<&'a[u8],HostAddress<'a>> {
    map_res!(
        i,
        parse_der_struct!(
            t: parse_der_tagged!(EXPLICIT 0, parse_der_int32) >>
            a: map_res!(parse_der_tagged!(EXPLICIT 1, parse_der_octetstring),|x: DerObject<'a>| x.as_slice()) >>
            ( HostAddress{
                addr_type: AddressType(t),
                address: a,
            })
        ),
        |(hdr,t) : (DerObjectHeader,HostAddress<'a>)| {
            if hdr.tag != DerTag::Sequence as u8 { return Err("not a sequence!"); }
            Ok(t)
        }
    )
}

/// Parse Kerberos HostAddresses
///
/// <pre>
/// -- NOTE: HostAddresses is always used as an OPTIONAL field and
/// -- should not be empty.
/// HostAddresses   -- NOTE: subtly different from rfc1510,
///                 -- but has a value mapping and encodes the same
///         ::= SEQUENCE OF HostAddress
/// </pre>
pub fn parse_krb5_hostaddresses<'a>(i: &'a[u8]) -> IResult<&'a[u8],Vec<HostAddress<'a>>> {
    map_res!(
        i,
        parse_der_struct!(
            v: many0!(parse_krb5_hostaddress) >>
            ( v )
        ),
        |(hdr,t) : (DerObjectHeader,_)| {
            if hdr.tag != DerTag::Sequence as u8 { return Err("not a sequence!"); }
            Ok(t)
        }
    )
}


/// Parse Kerberos Ticket
///
/// <pre>
/// Ticket          ::= [APPLICATION 1] SEQUENCE {
///         tkt-vno         [0] INTEGER (5),
///         realm           [1] Realm,
///         sname           [2] PrincipalName,
///         enc-part        [3] EncryptedData -- EncTicketPart
/// }
/// </pre>
pub fn parse_krb5_ticket<'a>(i: &'a[u8]) -> IResult<&'a[u8],Ticket<'a>> {
    parse_der_application!(
        i,
        APPLICATION 1,
        st: parse_der_struct!(
            no: parse_der_tagged!(EXPLICIT 0, parse_der_uint32) >>
                error_if!(no != 5, ErrorKind::Tag) >>
            r:  parse_der_tagged!(EXPLICIT 1, parse_krb5_realm) >>
            s:  parse_der_tagged!(EXPLICIT 2, parse_krb5_principalname) >>
            e:  map_res!(parse_der,|x: DerObject<'a>| x.as_slice()) >>
            ( Ticket{
                tkt_vno: no,
                realm: r,
                sname: s,
                enc_part: e
            })
        ) >> (st)
    ).map(|t| (t.1).1)
}

/// Parse Kerberos EncryptedData
///
/// <pre>
/// EncryptedData   ::= SEQUENCE {
///         etype   [0] Int32 -- EncryptionType --,
///         kvno    [1] UInt32 OPTIONAL,
///         cipher  [2] OCTET STRING -- ciphertext
/// }
/// </pre>
pub fn parse_encrypted<'a>(i:&'a[u8]) -> IResult<&'a[u8],EncryptedData<'a>> {
    parse_der_struct!(
        i,
        e: parse_der_tagged!(EXPLICIT 0, parse_der_int32) >>
        k: opt!(parse_der_tagged!(EXPLICIT 1, parse_der_uint32)) >>
        c: map_res!(parse_der_tagged!(EXPLICIT 2, parse_der_octetstring), |x: DerObject<'a>| x.as_slice()) >>
           eof!() >>
        ( EncryptedData{
            etype: EncryptionType(e),
            kvno: k,
            cipher: c
        })
    ).map(|t| t.1)
}


/// Parse a Kerberos KDC Request
///
/// <pre>
/// KDC-REQ         ::= SEQUENCE {
///         -- NOTE: first tag is [1], not [0]
///         pvno            [1] INTEGER (5) ,
///         msg-type        [2] INTEGER (10 -- AS -- | 12 -- TGS --),
///         padata          [3] SEQUENCE OF PA-DATA OPTIONAL
///                             -- NOTE: not empty --,
///         req-body        [4] KDC-REQ-BODY
/// }
/// </pre>
pub fn parse_kdc_req<'a>(i:&'a[u8]) -> IResult<&'a[u8],KdcReq<'a>> {
    parse_der_struct!(
        i,
        n: parse_der_tagged!(EXPLICIT 1, parse_der_uint32) >>
        t: parse_der_tagged!(EXPLICIT 2, parse_der_uint32) >>
        d: opt!(parse_der_tagged!(EXPLICIT 3, parse_krb5_padata_sequence)) >>
        b: parse_der_tagged!(EXPLICIT 4, parse_kdc_req_body) >>
           eof!() >>
        ( KdcReq{
            pvno: n,
            msg_type: MessageType(t),
            padata: d.unwrap_or(Vec::new()),
            req_body: b
        })
    ).map(|t| t.1)
}

/// Parse the body of a Kerberos KDC Request
///
/// <pre>
/// KDC-REQ-BODY    ::= SEQUENCE {
///         kdc-options             [0] KDCOptions,
///         cname                   [1] PrincipalName OPTIONAL
///                                     -- Used only in AS-REQ --,
///         realm                   [2] Realm
///                                     -- Server's realm
///                                     -- Also client's in AS-REQ --,
///         sname                   [3] PrincipalName OPTIONAL,
///         from                    [4] KerberosTime OPTIONAL,
///         till                    [5] KerberosTime,
///         rtime                   [6] KerberosTime OPTIONAL,
///         nonce                   [7] UInt32,
///         etype                   [8] SEQUENCE OF Int32 -- EncryptionType
///                                     -- in preference order --,
///         addresses               [9] HostAddresses OPTIONAL,
///         enc-authorization-data  [10] EncryptedData OPTIONAL
///                                     -- AuthorizationData --,
///         additional-tickets      [11] SEQUENCE OF Ticket OPTIONAL
///                                        -- NOTE: not empty
/// }
/// </pre>
pub fn parse_kdc_req_body<'a>(i:&'a[u8]) -> IResult<&'a[u8],KdcReqBody<'a>> {
    parse_der_struct!(
        i,
        o:     parse_der_tagged!(EXPLICIT 0,parse_kerberos_flags) >>
        cname: opt!(parse_der_tagged!(EXPLICIT 1,parse_krb5_principalname)) >>
        realm: parse_der_tagged!(EXPLICIT 2,parse_krb5_realm) >>
        sname: opt!(parse_der_tagged!(EXPLICIT 3,parse_krb5_principalname)) >>
        from:  opt!(parse_der_tagged!(EXPLICIT 4,parse_kerberos_time)) >>
        till:  parse_der_tagged!(EXPLICIT 5,parse_kerberos_time) >>
        rtime: opt!(complete!(parse_der_tagged!(EXPLICIT 6,parse_kerberos_time))) >>
        nonce: parse_der_tagged!(EXPLICIT 7, parse_der_uint32) >>
        etype: parse_der_tagged!(EXPLICIT 8,
                                 parse_der_struct!(v: many1!(parse_der_int32) >> (v.iter().map(|&x| EncryptionType(x)).collect()))
                                 ) >>
        addr:  opt!(complete!(parse_der_tagged!(9,parse_krb5_hostaddresses))) >>
        ead:   opt!(complete!(parse_der_tagged!(10,parse_encrypted))) >>
        atkts: opt!(complete!(parse_der_tagged!(EXPLICIT 11,
                                 parse_der_struct!(v: many1!(parse_krb5_ticket) >> (v))
                                 ))) >>
               eof!() >>
        ( KdcReqBody{
            kdc_options: o,
            cname: cname,
            realm: realm,
            sname: sname,
            from: from,
            till: till,
            rtime: rtime,
            nonce: nonce,
            etype: etype.1,
            addresses: addr.unwrap_or(vec![]),
            enc_authorization_data: ead,
            additional_tickets: if atkts.is_some() { atkts.unwrap().1 } else { vec![] }
        })
    ).map(|t| t.1)
}

/// Parse a Kerberos AS Request
///
/// <pre>
/// AS-REQ          ::= [APPLICATION 10] KDC-REQ
/// </pre>
pub fn parse_as_req<'a>(i:&'a[u8]) -> IResult<&'a[u8],KdcReq<'a>> {
    parse_der_application!(
        i,
        APPLICATION 10,
        req: parse_kdc_req >> (req)
    ).map(|t| t.1)
}

/// Parse a Kerberos TGS Request
///
/// <pre>
/// TGS-REQ          ::= [APPLICATION 12] KDC-REQ
/// </pre>
pub fn parse_tgs_req<'a>(i:&'a[u8]) -> IResult<&'a[u8],KdcReq<'a>> {
    parse_der_application!(
        i,
        APPLICATION 12,
        req: parse_kdc_req >> (req)
    ).map(|t| t.1)
}

/// Parse a Kerberos KDC Reply
///
/// <pre>
/// KDC-REP         ::= SEQUENCE {
///         pvno            [0] INTEGER (5),
///         msg-type        [1] INTEGER (11 -- AS -- | 13 -- TGS --),
///         padata          [2] SEQUENCE OF PA-DATA OPTIONAL
///                                 -- NOTE: not empty --,
///         crealm          [3] Realm,
///         cname           [4] PrincipalName,
///         ticket          [5] Ticket,
///         enc-part        [6] EncryptedData
///                                 -- EncASRepPart or EncTGSRepPart,
///                                 -- as appropriate
/// }
/// </pre>
pub fn parse_kdc_rep<'a>(i:&'a[u8]) -> IResult<&'a[u8],KdcRep<'a>> {
    parse_der_struct!(
        i,
        pvno:    parse_der_tagged!(EXPLICIT 0,parse_der_uint32) >>
        msgtype: parse_der_tagged!(EXPLICIT 1,parse_der_uint32) >>
        padata:  opt!(parse_der_tagged!(EXPLICIT 2,parse_krb5_padata_sequence)) >>
        crealm:  parse_der_tagged!(EXPLICIT 3,parse_krb5_realm) >>
        cname:   parse_der_tagged!(EXPLICIT 4,parse_krb5_principalname) >>
        ticket:  parse_der_tagged!(EXPLICIT 5,parse_krb5_ticket) >>
        encp:    parse_der_tagged!(EXPLICIT 6,parse_encrypted) >>
               eof!() >>
        ( KdcRep{
            pvno: pvno,
            msg_type: MessageType(msgtype),
            padata: padata.unwrap_or(Vec::new()),
            crealm: crealm,
            cname: cname,
            ticket: ticket,
            enc_part: encp,
        })
    ).map(|t| t.1)
}

/// Parse a Kerberos AS Reply
///
/// <pre>
/// AS-REP          ::= [APPLICATION 11] KDC-REP
/// </pre>
pub fn parse_as_rep<'a>(i:&'a[u8]) -> IResult<&'a[u8],KdcRep<'a>> {
    parse_der_application!(
        i,
        APPLICATION 11,
        rep: parse_kdc_rep >> (rep)
    ).map(|t| t.1)
}

/// Parse a Kerberos TGS Reply
///
/// <pre>
/// TGS-REP          ::= [APPLICATION 13] KDC-REP
/// </pre>
pub fn parse_tgs_rep<'a>(i:&'a[u8]) -> IResult<&'a[u8],KdcRep<'a>> {
    parse_der_application!(
        i,
        APPLICATION 13,
        rep: parse_kdc_rep >> (rep)
    ).map(|t| t.1)
}

/// Parse a Kerberos Error
///
/// <pre>
/// KRB-ERROR       ::= [APPLICATION 30] SEQUENCE {
///         pvno            [0] INTEGER (5),
///         msg-type        [1] INTEGER (30),
///         ctime           [2] KerberosTime OPTIONAL,
///         cusec           [3] Microseconds OPTIONAL,
///         stime           [4] KerberosTime,
///         susec           [5] Microseconds,
///         error-code      [6] Int32,
///         crealm          [7] Realm OPTIONAL,
///         cname           [8] PrincipalName OPTIONAL,
///         realm           [9] Realm -- service realm --,
///         sname           [10] PrincipalName -- service name --,
///         e-text          [11] KerberosString OPTIONAL,
///         e-data          [12] OCTET STRING OPTIONAL
/// }
/// </pre>
pub fn parse_krb_error<'a>(i:&'a[u8]) -> IResult<&'a[u8],KrbError<'a>> {
    parse_der_application!(
        i,
        APPLICATION 30,
        st: parse_der_struct!(
            pvno:    parse_der_tagged!(EXPLICIT 0,parse_der_uint32) >>
                     error_if!(pvno != 5, ErrorKind::Tag) >>
            msgtype: parse_der_tagged!(EXPLICIT 1,parse_der_uint32) >>
                     error_if!(msgtype != 30, ErrorKind::Tag) >>
            ctime:   opt!(parse_der_tagged!(EXPLICIT 2,parse_kerberos_time)) >>
            cusec:   opt!(parse_der_tagged!(EXPLICIT 3,parse_der_microseconds)) >>
            stime:   parse_der_tagged!(EXPLICIT 4,parse_kerberos_time) >>
            susec:   parse_der_tagged!(EXPLICIT 5,parse_der_microseconds) >>
            errorc:  parse_der_tagged!(EXPLICIT 6,parse_der_int32) >>
            crealm:  opt!(parse_der_tagged!(EXPLICIT 7,parse_krb5_realm)) >>
            cname:   opt!(parse_der_tagged!(EXPLICIT 8,parse_krb5_principalname)) >>
            realm:   parse_der_tagged!(EXPLICIT 9,parse_krb5_realm) >>
            sname:   parse_der_tagged!(EXPLICIT 10,parse_krb5_principalname) >>
            etext:   opt!(complete!(parse_der_tagged!(EXPLICIT 11,parse_kerberos_string))) >>
            edata:   opt!(complete!(parse_der_tagged!(EXPLICIT 12,parse_der_octetstring))) >>
            (KrbError{
                pvno: pvno,
                msg_type: MessageType(msgtype),
                ctime: ctime,
                cusec: cusec,
                stime: stime,
                susec: susec,
                error_code: errorc as i32, // XXX i32
                crealm: crealm,
                cname: cname,
                realm: realm,
                sname: sname,
                etext: etext,
                edata: edata,
            })
        )
        >> (st)
    ).map(|t| (t.1).1)
}

/// Parse Kerberos PA-Data
///
/// <pre>
/// PA-DATA         ::= SEQUENCE {
///         -- NOTE: first tag is [1], not [0]
///         padata-type     [1] Int32,
///         padata-value    [2] OCTET STRING -- might be encoded AP-REQ
/// }
/// </pre>
pub fn parse_krb5_padata<'a>(i: &'a[u8]) -> IResult<&'a[u8],PAData<'a>> {
    map_res!(
        i,
        parse_der_struct!(
            t: dbg_dmp!(parse_der_tagged!(EXPLICIT 1, parse_der_int32)) >>
            s: map_res!(parse_der_tagged!(EXPLICIT 2, parse_der_octetstring),|x: DerObject<'a>| x.as_slice()) >>
            ( PAData{
                padata_type:  PAType(t),
                padata_value: s,
            })
        ),
        |(hdr,t) : (DerObjectHeader,_)| {
            if hdr.tag != DerTag::Sequence as u8 { return Err("not a sequence!"); }
            Ok(t)
        }
    )
}

fn parse_krb5_padata_sequence(i: &[u8]) -> IResult<&[u8],Vec<PAData>> {
    map_res!(
        i,
        parse_der_struct!(
            v: many0!(parse_krb5_padata) >>
            ( v )
        ),
        |(hdr,t) : (DerObjectHeader,_)| {
            if hdr.tag != DerTag::Sequence as u8 { return Err("not a sequence!"); }
            Ok(t)
        }
    )
}
