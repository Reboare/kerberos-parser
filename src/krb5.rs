//! Kerberos 5 structures
//!
//! - [RFC1510](https://tools.ietf.org/html/rfc1510) The Kerberos Network Authentication Service (V5)
//! - [RFC3961](https://tools.ietf.org/html/rfc3961) Encryption and Checksum Specifications for Kerberos 5
//! - [RFC3962](https://tools.ietf.org/html/rfc3962) Advanced Encryption Standard (AES) Encryption for Kerberos 5
//! - [RFC4120](https://tools.ietf.org/html/rfc4120) The Kerberos Network Authentication Service (V5)
//! - [RFC6803](https://tools.ietf.org/html/rfc6803) Camellia Encryption for Kerberos 5
//! - [RFC8009](https://tools.ietf.org/html/rfc8009) AES Encryption with HMAC-SHA2 for Kerberos 5

use der_parser::der::DerObject;
use std::fmt;

pub use crate::krb5_constants::*;
pub use crate::krb5_errors::*;

/// Kerberos Realm
///
/// A Kerberos realm is a set of managed nodes that share the same Kerberos database.
#[derive(Debug, PartialEq, Clone)]
pub struct Realm(pub String);

/// Kerberos PrincipalName
///
/// A Kerberos principal is a service or user that is known to the Kerberos system. Each Kerberos
/// principal is identified by its principal name. Principal names consist of three parts: a
/// service or user name, an instance name, and a realm name in the following form:
///
/// <pre>
/// principal-name.instance-name@realm-name
/// </pre>
#[derive(Debug, PartialEq, Clone)]
pub struct PrincipalName {
    pub name_type: NameType,
    pub name_string: Vec<String>,
}

impl fmt::Display for PrincipalName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.name_string.join("/"))
    }
}

/// Kerberos Ticket
///
/// A record that helps a client authenticate itself to a server; it
/// contains the client's identity, a session key, a timestamp, and
/// other information, all sealed using the server's secret key.  It
/// only serves to authenticate a client when presented along with a
/// fresh Authenticator.
#[derive(Debug, PartialEq, Clone)]
pub struct Ticket<'a> {
    /// Version number for the ticket format (5)
    pub tkt_vno: u32,
    /// Realm that issued a ticket
    pub realm: Realm,
    /// Components of the name part of the server's identity
    pub sname: PrincipalName,
    /// Encrypted encoding of the EncTicketPart sequence
    pub enc_part: Cow<'a, EncryptedData<'a>>,
}
use std::borrow::Cow;
/// Kerberos EncryptedData
#[derive(Clone, Debug, PartialEq)]
pub struct EncryptedData<'a> {
    /// EncryptionType
    pub etype: EncryptionType,
    /// Version number of the key under which data is encrypted
    pub kvno: Option<u32>,
    /// Ciphertext
    pub cipher: Cow<'a, [u8]>,
}

/// Key Distribution Center (KDC) Request Message
#[derive(Debug, PartialEq)]
pub struct KdcReq<'a> {
    pub pvno: u32,
    pub msg_type: MessageType,
    pub padata: Vec<PAData<'a>>,
    pub req_body: KdcReqBody<'a>,
}

/// Key Distribution Center (KDC) Request Message Body
#[derive(Debug, PartialEq)]
pub struct KdcReqBody<'a> {
    /// Options requested by the client
    pub kdc_options: DerObject<'a>,
    /// Client name (only for AS-REQ)
    pub cname: Option<PrincipalName>,
    /// Server's realm
    pub realm: Realm,
    /// Server name
    pub sname: Option<PrincipalName>,
    /// Desired starttime for the requested ticket
    pub from: Option<DerObject<'a>>,
    /// Expiration date requested by the client
    pub till: DerObject<'a>,
    /// Requested renew-till time
    pub rtime: Option<DerObject<'a>>,
    /// Random number generated by the client
    pub nonce: u32,
    /// Desired encryption algorithm to be used in the response
    pub etype: Vec<EncryptionType>,
    /// Addresses from which the requested ticket is to be valid
    pub addresses: Vec<HostAddress<'a>>,
    /// Encoding of the desired authorization-data encrypted under the sub-session key if present
    /// in the Authenticator, or alternatively from the session key in the TGT
    pub enc_authorization_data: Option<EncryptedData<'a>>,
    /// Additional tickets MAY be optionally included in a request to the ticket-granting server
    pub additional_tickets: Vec<Ticket<'a>>,
}

/// Kerberos HostAddress
#[derive(Debug, PartialEq)]
pub struct HostAddress<'a> {
    pub addr_type: AddressType,
    pub address: &'a [u8],
}

/// Key Distribution Center (KDC) Reply Message
#[derive(Debug, PartialEq)]
pub struct KdcRep<'a> {
    pub pvno: u32,
    pub msg_type: MessageType,
    pub padata: Vec<PAData<'a>>,
    pub crealm: Realm,
    pub cname: PrincipalName,
    pub ticket: Ticket<'a>,
    pub enc_part: EncryptedData<'a>,
}

/// Kerberos Error message
#[derive(Debug, PartialEq)]
pub struct KrbError<'a> {
    pub pvno: u32,
    pub msg_type: MessageType,
    pub ctime: Option<DerObject<'a>>,
    pub cusec: Option<u32>,
    pub stime: DerObject<'a>,
    pub susec: u32,
    pub error_code: ErrorCode,
    pub crealm: Option<Realm>,
    pub cname: Option<PrincipalName>,
    pub realm: Realm,
    pub sname: PrincipalName,
    pub etext: Option<String>,
    pub edata: Option<DerObject<'a>>,
}

/// Kerberos PA-Data
#[derive(Debug, PartialEq)]
pub struct PAData<'a> {
    pub padata_type: PAType,
    pub padata_value: &'a [u8],
}

/// Kerberos AP Request
#[derive(Debug, PartialEq, Clone)]
pub struct ApReq<'a> {
    pub pvno: u32,
    pub msg_type: MessageType,
    pub ap_options: DerObject<'a>, // KerberosFlags
    pub ticket: Ticket<'a>,
    pub authenticator: Cow<'a, EncryptedData<'a>>,
}

/// Kerberos AP Reply
#[derive(Debug, PartialEq, Clone)]
pub struct ApRep<'a> {
    pub pvno: u32,
    pub msg_type: MessageType,
    pub enc_part: Cow<'a, EncryptedData<'a>>,
}
