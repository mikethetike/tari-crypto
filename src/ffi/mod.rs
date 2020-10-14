// Copyright 2020. The Tari Project
// Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
// following conditions are met:
// 1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
// disclaimer.
// 2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
// following disclaimer in the documentation and/or other materials provided with the distribution.
// 3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
// products derived from this software without specific prior written permission.
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
// INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
// WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
// USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use crate::{
    hash::blake2::Blake256,
    keys::{PublicKey, SecretKey},
    ristretto::{RistrettoPublicKey, RistrettoSchnorr, RistrettoSecretKey},
};
use digest::Digest;
use libc::c_char;
use rand::rngs::OsRng;
use std::ffi::CStr;
use tari_utilities::ByteArray;

const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\u{00}");

#[no_mangle]
pub extern "C" fn version() -> *const c_char {
    VERSION.as_ptr() as *const c_char
}

const KEY_LENGTH: usize = 32;

type KeyArray = [u8; KEY_LENGTH];

/// Generate a new key pair and copies the values into the provided arrays.
///
/// If `pub_key` is null, then only a private key is generated.
/// The *caller* must manage memory for the results. Besides checking for null values, this function assumes that at
/// least `KEY_LENGTH` bytes have been allocated in `priv_key` and `pub_key`.
#[no_mangle]
pub unsafe extern "C" fn random_keypair(priv_key: *mut KeyArray, pub_key: *mut KeyArray) {
    if priv_key.is_null() && pub_key.is_null() {
        return;
    }
    if pub_key.is_null() {
        let k = RistrettoSecretKey::random(&mut OsRng);
        convert_to_c_array(k.as_bytes(), priv_key);
    } else {
        let (k, p) = RistrettoPublicKey::random_keypair(&mut OsRng);
        convert_to_c_array(k.as_bytes(), priv_key);
        convert_to_c_array(p.as_bytes(), pub_key);
    }
}

#[no_mangle]
pub unsafe extern "C" fn sign(
    priv_key: *const KeyArray,
    msg: *const c_char,
    nonce: *mut KeyArray,
    signature: *mut KeyArray,
)
{
    if nonce.is_null() || signature.is_null() || priv_key.is_null() || msg.is_null() {
        return;
    }
    let k = match RistrettoSecretKey::from_bytes(&(*priv_key)) {
        Ok(k) => k,
        _ => return,
    };
    let r = RistrettoSecretKey::random(&mut OsRng);
    let msg = match CStr::from_ptr(msg).to_str() {
        Ok(s) => s,
        _ => return,
    };
    let challenge = Blake256::digest(msg.as_bytes()).to_vec();
    let sig = match RistrettoSchnorr::sign(k, r, &challenge) {
        Ok(sig) => sig,
        _ => return,
    };
    convert_to_c_array(sig.get_public_nonce().as_bytes(), nonce);
    convert_to_c_array(sig.get_signature().as_bytes(), signature);
}

#[no_mangle]
pub unsafe extern "C" fn verify(
    pub_key: *const KeyArray,
    msg: *const c_char,
    pub_nonce: *mut KeyArray,
    signature: *mut KeyArray,
) -> bool
{
    if pub_key.is_null() || msg.is_null() || pub_nonce.is_null() || signature.is_null() {
        return false;
    }
    let pk = match RistrettoPublicKey::from_bytes(&(*pub_key)) {
        Ok(k) => k,
        _ => return false,
    };
    let r_pub = match RistrettoPublicKey::from_bytes(&(*pub_nonce)) {
        Ok(r) => r,
        _ => return false,
    };
    let sig = match RistrettoSecretKey::from_bytes(&(*signature)) {
        Ok(s) => s,
        _ => return false,
    };
    let msg = match CStr::from_ptr(msg).to_str() {
        Ok(s) => s,
        _ => return false,
    };
    let sig = RistrettoSchnorr::new(r_pub, sig);
    let challenge = Blake256::digest(msg.as_bytes());
    let challenge = match RistrettoSecretKey::from_bytes(challenge.as_slice()) {
        Ok(e) => e,
        _ => return false,
    };
    sig.verify(&pk, &challenge)
}

unsafe fn convert_to_c_array(src: &[u8], trg: *mut KeyArray) {
    (*trg).copy_from_slice(src);
}
