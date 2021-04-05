/*!
# Secret-key authenticated encryption

_For public-key based encryption, see [dryocbox](crate::dryocbox)_.

_For stream encryption, see [dryocstream](crate::dryocstream)_.

# Rustaceous API example

```
use dryoc::dryocsecretbox::*;

let secret_key = Key::gen();
let nonce = Nonce::gen();
let message = b"hey";

// Must specify return type
let dryocsecretbox = DryocSecretBox::encrypt_to_vecbox(message, &nonce, &secret_key);

let decrypted = dryocsecretbox
    .decrypt_to_vec(&nonce, &secret_key)
    .expect("unable to decrypt");

assert_eq!(message, decrypted.as_slice());
```
*/

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::constants::{
    CRYPTO_SECRETBOX_KEYBYTES, CRYPTO_SECRETBOX_MACBYTES, CRYPTO_SECRETBOX_NONCEBYTES,
};
use crate::error::Error;
pub use crate::types::*;

/// Container for crypto secret box message authentication code.
pub type Mac = StackByteArray<CRYPTO_SECRETBOX_MACBYTES>;
/// A nonce for secret key authenticated boxes.
pub type Nonce = StackByteArray<CRYPTO_SECRETBOX_NONCEBYTES>;
/// A secret for secret key authenticated boxes.
pub type Key = StackByteArray<CRYPTO_SECRETBOX_KEYBYTES>;

#[cfg(any(feature = "nightly", all(doc, not(doctest))))]
#[cfg_attr(all(feature = "nightly", doc), doc(cfg(feature = "nightly")))]
/// Type aliases for using protected memory with [DryocSecretBox].
pub mod protected {
    use super::*;
    pub use crate::protected::*;

    /// A secret for authenticated secret streams.
    pub type Key = HeapByteArray<CRYPTO_SECRETBOX_KEYBYTES>;
    /// A nonce for authenticated secret streams.
    pub type Nonce = HeapByteArray<CRYPTO_SECRETBOX_NONCEBYTES>;
    /// Container for crypto secret box message authentication code.
    pub type Mac = HeapByteArray<CRYPTO_SECRETBOX_MACBYTES>;

    pub type LockedBox = DryocSecretBox<t::Locked<Mac>, t::Locked<HeapBytes>>;
}

#[cfg_attr(
    feature = "serde",
    derive(Zeroize, Clone, Debug, Serialize, Deserialize)
)]
#[cfg_attr(not(feature = "serde"), derive(Zeroize, Clone, Debug))]
/// A libsodium public-key authenticated encrypted box
pub struct DryocSecretBox<Mac: ByteArray<CRYPTO_SECRETBOX_MACBYTES>, Data: Bytes> {
    /// libsodium box authentication tag, usually prepended to each box
    tag: Mac,
    /// libsodium box message or ciphertext, depending on state
    data: Data,
}

pub type VecBox = DryocSecretBox<Mac, Vec<u8>>;

impl<Mac: NewByteArray<CRYPTO_SECRETBOX_MACBYTES>, Data: NewBytes> DryocSecretBox<Mac, Data> {
    /// Returns an empty box
    pub fn new() -> Self {
        Self {
            tag: Mac::new_byte_array(),
            data: Data::new_bytes(),
        }
    }
}

impl<Mac: NewByteArray<CRYPTO_SECRETBOX_MACBYTES>, Data: NewBytes + ResizableBytes>
    DryocSecretBox<Mac, Data>
{
    /// Encrypts a message using `sender_secret_key` for `recipient_public_key`,
    /// and returns a new [DryocSecretBox] with ciphertext and tag
    pub fn encrypt<
        Message: Bytes + ?Sized,
        Nonce: ByteArray<CRYPTO_SECRETBOX_NONCEBYTES>,
        SecretKey: ByteArray<CRYPTO_SECRETBOX_KEYBYTES>,
    >(
        message: &Message,
        nonce: &Nonce,
        secret_key: &SecretKey,
    ) -> Self {
        use crate::crypto_secretbox::crypto_secretbox_detached;

        let mut new = Self::new();
        new.data.resize(message.len(), 0);

        crypto_secretbox_detached(
            new.data.as_mut_slice(),
            new.tag.as_mut_array(),
            message.as_slice(),
            nonce.as_array(),
            secret_key.as_array(),
        );

        new
    }
}

impl<
    'a,
    Mac: ByteArray<CRYPTO_SECRETBOX_MACBYTES> + std::convert::TryFrom<&'a [u8]>,
    Data: Bytes + From<&'a [u8]>,
> DryocSecretBox<Mac, Data>
{
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() < CRYPTO_SECRETBOX_MACBYTES {
            Err(dryoc_error!(format!(
                "bytes of len {} less than expected minimum of {}",
                bytes.len(),
                CRYPTO_SECRETBOX_MACBYTES
            )))
        } else {
            let (tag, data) = bytes.split_at(CRYPTO_SECRETBOX_MACBYTES);
            Ok(Self {
                tag: Mac::try_from(tag).map_err(|e| dryoc_error!("invalid tag"))?,
                data: Data::from(data),
            })
        }
    }
}

impl<Mac: ByteArray<CRYPTO_SECRETBOX_MACBYTES>, Data: Bytes> DryocSecretBox<Mac, Data> {
    /// Returns a new box with `tag` and `data`, consuming both
    pub fn from_data_and_mac(tag: Mac, data: Data) -> Self {
        Self { tag, data }
    }
}

impl<Mac: ByteArray<CRYPTO_SECRETBOX_MACBYTES>, Data: Bytes> DryocSecretBox<Mac, Data> {
    /// Decrypts `ciphertext` using `recipient_secret_key` and
    /// `sender_public_key`, returning a new [DryocSecretBox] with decrypted
    /// message
    pub fn decrypt<
        Output: ResizableBytes + NewBytes,
        Nonce: ByteArray<CRYPTO_SECRETBOX_NONCEBYTES>,
        SecretKey: ByteArray<CRYPTO_SECRETBOX_KEYBYTES>,
    >(
        &self,
        nonce: &Nonce,
        secret_key: &SecretKey,
    ) -> Result<Output, Error> {
        use crate::crypto_secretbox::crypto_secretbox_open_detached;

        let mut message = Output::new_bytes();
        message.resize(self.data.as_slice().len(), 0);

        crypto_secretbox_open_detached(
            message.as_mut_slice(),
            self.tag.as_array(),
            self.data.as_slice(),
            nonce.as_array(),
            secret_key.as_array(),
        )?;

        Ok(message)
    }

    /// Copies this box into a new Vec
    pub fn to_vec(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(self.tag.as_array());
        data.extend(self.data.as_slice());
        data
    }
}

impl DryocSecretBox<Mac, Vec<u8>> {
    pub fn new_vecbox() -> Self {
        Self::new()
    }

    /// Encrypts a message using `sender_secret_key` for `recipient_public_key`,
    /// and returns a new [DryocSecretBox] with ciphertext and tag
    pub fn encrypt_to_vecbox<
        Message: Bytes + ?Sized,
        Nonce: ByteArray<CRYPTO_SECRETBOX_NONCEBYTES>,
        SecretKey: ByteArray<CRYPTO_SECRETBOX_KEYBYTES>,
    >(
        message: &Message,
        nonce: &Nonce,
        secret_key: &SecretKey,
    ) -> Self {
        Self::encrypt(message, nonce, secret_key)
    }

    /// Decrypts `ciphertext` using `recipient_secret_key` and
    /// `sender_public_key`, returning a new [DryocSecretBox] with decrypted
    /// message
    pub fn decrypt_to_vec<
        Nonce: ByteArray<CRYPTO_SECRETBOX_NONCEBYTES>,
        SecretKey: ByteArray<CRYPTO_SECRETBOX_KEYBYTES>,
    >(
        &self,
        nonce: &Nonce,
        secret_key: &SecretKey,
    ) -> Result<Vec<u8>, Error> {
        self.decrypt(nonce, secret_key)
    }

    /// Consumes this box and returns it as a Vec
    pub fn into_vec(mut self) -> Vec<u8> {
        self.data
            .resize(self.data.len() + CRYPTO_SECRETBOX_MACBYTES, 0);
        self.data.rotate_right(CRYPTO_SECRETBOX_MACBYTES);
        self.data[0..CRYPTO_SECRETBOX_MACBYTES].copy_from_slice(self.tag.as_array());
        self.data
    }
}

impl<
    'a,
    Mac: NewByteArray<CRYPTO_SECRETBOX_MACBYTES>,
    Data: NewBytes + ResizableBytes + From<&'a [u8]>,
> DryocSecretBox<Mac, Data>
{
    /// Returns a box with `data` copied from slice `input`.
    pub fn with_data(input: &'a [u8]) -> Self {
        Self {
            tag: Mac::new_byte_array(),
            data: input.into(),
        }
    }
}

impl<
    'a,
    Mac: ByteArray<CRYPTO_SECRETBOX_MACBYTES>,
    Data: NewBytes + ResizableBytes + From<&'a [u8]>,
> DryocSecretBox<Mac, Data>
{
    /// Returns a new box with `data` and `tag`, with data copied from `input`
    /// and `tag` consumed.
    pub fn with_data_and_mac(tag: Mac, input: &'a [u8]) -> Self {
        Self {
            tag,
            data: input.into(),
        }
    }
}

impl<Mac: NewByteArray<CRYPTO_SECRETBOX_MACBYTES>, Data: NewBytes + ResizableBytes> Default
    for DryocSecretBox<Mac, Data>
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_eq<T>(t: &[T], v: T) -> bool
    where
        T: PartialEq,
    {
        t.iter().fold(true, |acc, x| acc && *x == v)
    }

    #[test]
    fn test_new() {
        let dryocsecretbox: VecBox = DryocSecretBox::new();

        assert_eq!(all_eq(&dryocsecretbox.tag, 0), true);
        assert_eq!(all_eq(&dryocsecretbox.data, 0), true);

        let dryocsecretbox = DryocSecretBox::new_vecbox();

        assert_eq!(all_eq(&dryocsecretbox.tag, 0), true);
        assert_eq!(all_eq(&dryocsecretbox.data, 0), true);
    }

    #[test]
    fn test_default() {
        let dryocsecretbox: VecBox = DryocSecretBox::default();

        assert_eq!(all_eq(dryocsecretbox.tag.as_slice(), 0), true);
        assert_eq!(all_eq(&dryocsecretbox.data, 0), true);
    }

    #[test]
    fn test_dryocbox() {
        for i in 0..20 {
            use base64::encode;
            use sodiumoxide::crypto::secretbox;
            use sodiumoxide::crypto::secretbox::{Key as SOKey, Nonce as SONonce};

            use crate::dryocsecretbox::*;

            let secret_key = Key::gen();
            let nonce = Nonce::gen();
            let words = vec!["hello1".to_string(); i];
            let message = words.join(" :D ").into_bytes();
            let message_copy = message.clone();
            let dryocsecretbox: VecBox = DryocSecretBox::encrypt(&message, &nonce, &secret_key);

            let ciphertext = dryocsecretbox.clone().into_vec();
            assert_eq!(&ciphertext, &dryocsecretbox.to_vec());

            let ciphertext_copy = ciphertext.clone();

            let so_ciphertext = secretbox::seal(
                &message_copy,
                &SONonce::from_slice(&nonce).unwrap(),
                &SOKey::from_slice(&secret_key).unwrap(),
            );
            assert_eq!(encode(&ciphertext), encode(&so_ciphertext));

            let so_decrypted = secretbox::open(
                &ciphertext_copy,
                &SONonce::from_slice(&nonce).unwrap(),
                &SOKey::from_slice(&secret_key).unwrap(),
            )
            .expect("decrypt failed");

            let m = DryocSecretBox::decrypt::<Vec<u8>, Nonce, Key>(
                &dryocsecretbox,
                &nonce,
                &secret_key,
            )
            .expect("decrypt failed");
            assert_eq!(m, message_copy);
            assert_eq!(m, so_decrypted);
        }
    }

    #[test]
    fn test_dryocbox_vec() {
        for i in 0..20 {
            use base64::encode;
            use sodiumoxide::crypto::secretbox;
            use sodiumoxide::crypto::secretbox::{Key as SOKey, Nonce as SONonce};

            use crate::dryocsecretbox::*;

            let secret_key = Key::gen();
            let nonce = Nonce::gen();
            let words = vec!["hello1".to_string(); i];
            let message = words.join(" :D ").into_bytes();
            let message_copy = message.clone();
            let dryocsecretbox = DryocSecretBox::encrypt_to_vecbox(&message, &nonce, &secret_key);

            let ciphertext = dryocsecretbox.clone().into_vec();
            assert_eq!(&ciphertext, &dryocsecretbox.to_vec());

            let ciphertext_copy = ciphertext.clone();

            let so_ciphertext = secretbox::seal(
                &message_copy,
                &SONonce::from_slice(&nonce).unwrap(),
                &SOKey::from_slice(&secret_key).unwrap(),
            );
            assert_eq!(encode(&ciphertext), encode(&so_ciphertext));

            let so_decrypted = secretbox::open(
                &ciphertext_copy,
                &SONonce::from_slice(&nonce).unwrap(),
                &SOKey::from_slice(&secret_key).unwrap(),
            )
            .expect("decrypt failed");

            let m = dryocsecretbox
                .decrypt_to_vec(&nonce, &secret_key)
                .expect("decrypt failed");
            assert_eq!(m, message_copy);
            assert_eq!(m, so_decrypted);
        }
    }

    #[test]
    fn test_copy() {
        for _ in 0..20 {
            use std::convert::TryFrom;

            use crate::rng::*;

            let mut data1: Vec<u8> = vec![0u8; 1024];
            copy_randombytes(data1.as_mut_slice());
            let data1_copy = data1.clone();

            let dryocsecretbox: VecBox = DryocSecretBox::from_bytes(&data1).expect("ok");
            assert_eq!(
                dryocsecretbox.data.as_slice(),
                &data1_copy[CRYPTO_SECRETBOX_MACBYTES..]
            );
            assert_eq!(
                dryocsecretbox.tag.as_slice(),
                &data1_copy[..CRYPTO_SECRETBOX_MACBYTES]
            );

            let data1 = data1_copy.clone();
            let dryocsecretbox: VecBox = DryocSecretBox::with_data(&data1);
            assert_eq!(&dryocsecretbox.data, &data1_copy);

            let data1 = data1_copy.clone();
            let (tag, data) = data1.split_at(CRYPTO_SECRETBOX_MACBYTES);
            let dryocsecretbox: VecBox =
                DryocSecretBox::with_data_and_mac(Mac::try_from(tag).expect("mac"), data);
            assert_eq!(
                dryocsecretbox.data.as_slice(),
                &data1_copy[CRYPTO_SECRETBOX_MACBYTES..]
            );
            assert_eq!(
                dryocsecretbox.tag.as_array(),
                &data1_copy[..CRYPTO_SECRETBOX_MACBYTES]
            );
        }
    }

    #[cfg(any(feature = "nightly", all(doc, not(doctest))))]
    #[cfg(feature = "nightly")]
    #[test]
    fn test_dryocbox_locked() {
        for i in 0..20 {
            use base64::encode;
            use sodiumoxide::crypto::secretbox;
            use sodiumoxide::crypto::secretbox::{Key as SOKey, Nonce as SONonce};

            use crate::dryocsecretbox::*;
            use crate::protected::*;

            let secret_key = protected::Key::gen_locked().expect("gen failed");
            let nonce = protected::Nonce::gen_locked().expect("gen failed");
            let words = vec!["hello1".to_string(); i];
            let message = words.join(" :D ");
            let message_copy = message.clone();
            let dryocsecretbox: protected::LockedBox =
                DryocSecretBox::encrypt(message.as_bytes(), &nonce, &secret_key);

            let ciphertext = dryocsecretbox.to_vec();

            let ciphertext_copy = ciphertext.clone();

            let so_ciphertext = secretbox::seal(
                &message_copy.as_bytes(),
                &SONonce::from_slice(nonce.as_slice()).unwrap(),
                &SOKey::from_slice(secret_key.as_slice()).unwrap(),
            );
            assert_eq!(encode(&ciphertext), encode(&so_ciphertext));

            let so_decrypted = secretbox::open(
                &ciphertext_copy,
                &SONonce::from_slice(nonce.as_slice()).unwrap(),
                &SOKey::from_slice(secret_key.as_slice()).unwrap(),
            )
            .expect("decrypt failed");

            let m: t::LockedBytes = dryocsecretbox
                .decrypt(&nonce, &secret_key)
                .expect("decrypt failed");

            assert_eq!(m.as_slice(), message_copy.as_bytes());
            assert_eq!(m.as_slice(), so_decrypted);
        }
    }
}
