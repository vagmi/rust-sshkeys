use std::io::Read;
use std::fs::File;
use std::path::Path;

use super::keytype::KeyType;
use super::cursor::Cursor;
use super::error::{Error, Kind, Result};

use base64;

// The different kinds of public keys.
pub enum PublicKeyKind {
    Rsa(RsaPublicKey),
}

// TODO: Implement methods on `PublicKeyKind` for displaying key fingerprint

// RSA public key format is described in RFC 4253, section 6.6
pub struct RsaPublicKey {
    pub e: Vec<u8>,
    pub n: Vec<u8>,
}

// Represents a public key in OpenSSH format
pub struct PublicKey {
    pub key_type: KeyType,
    pub kind: PublicKeyKind,
    pub comment: Option<String>,
}

impl PublicKey {
    // TODO: Implement method for displaying the key bits
    // TODO: Implement method for displaying the key fingerprint

    // Reads an OpenSSH public key from a given path.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<PublicKey> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        PublicKey::from_string(&contents)
    }

    // Reads an OpenSSH public key from the given string.
    pub fn from_string(contents: &str) -> Result<PublicKey> {
        let mut iter = contents.split_whitespace();

        let kt_name = iter.next().ok_or(Error::with_kind(Kind::InvalidFormat))?;
        let kt = KeyType::from_name(&kt_name)?;

        let data = iter.next().ok_or(Error::with_kind(Kind::InvalidFormat))?;
        let comment = iter.next().map(|v| String::from(v));

        let decoded = base64::decode(&data)?;
        let mut cursor = Cursor::new(&decoded);

        // Validate key format before reading rest of the data
        let kt_from_cursor = cursor.read_string()?;
        if kt_name != kt_from_cursor {
            return Err(Error::with_kind(Kind::KeyTypeMismatch))
        }

        let kind = PublicKey::from_cursor(&kt, &mut cursor)?;
        let key = PublicKey {
            key_type: kt,
            kind: kind,
            comment: comment,
        };

        Ok(key)
    }

    // Reads a public key from the given byte sequence, e.g. a public key extracted
    // from an OpenSSH certificate.
    // The byte sequence is expected to be the base64 decoded body of the public key.
    pub fn from_bytes<T: ?Sized + AsRef<[u8]>>(data: &T) -> Result<PublicKey> {
        let mut cursor = Cursor::new(&data);
        let kt_name = cursor.read_string()?;
        let kt = KeyType::from_name(&kt_name)?;
        let kind = PublicKey::from_cursor(&kt, &mut cursor)?;
        let key = PublicKey {
            key_type: kt,
            kind: kind,
            comment: None,
        };

        Ok(key)
    }

    // This function is used for extracting a public key from an existing cursor, e.g.
    // we already have a cursor for reading an OpenSSH certificate key and
    // we want to extract the public key information from it.
    pub(crate) fn from_cursor(kt: &KeyType, cursor: &mut Cursor) -> Result<PublicKeyKind> {
        let name = kt.name();
        let kind = match name {
            "ssh-rsa" |
            "ssh-rsa-cert-v01@openssh.com" => {
                let k = RsaPublicKey {
                    e: cursor.read_mpint()?,
                    n: cursor.read_mpint()?,
                };

                PublicKeyKind::Rsa(k)
            },
            // TODO: Implement the rest of the key kinds
            _ => return Err(Error::with_kind(Kind::UnknownKeyType(String::from(name)))),
        };

        // TODO: This should probably return a new `PublicKey` instead of `PublicKeyKind`

        Ok(kind)
    }
}