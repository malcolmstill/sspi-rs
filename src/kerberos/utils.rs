use std::io::Write;

use picky_krb::constants::key_usages::INITIATOR_SIGN;
use picky_krb::crypto::aes::{checksum_sha_aes, AesSize};
use picky_krb::gss_api::MicToken;
use serde::Serialize;

use crate::kerberos::client::generators::get_mech_list;
use crate::kerberos::encryption_params::EncryptionParams;
use crate::{Error, ErrorKind, Result};

pub fn serialize_message<T: ?Sized + Serialize>(v: &T) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    // 4 bytes: length of the message
    data.write_all(&[0, 0, 0, 0])?;

    picky_asn1_der::to_writer(v, &mut data)?;

    let len = data.len() as u32 - 4;
    data[0..4].copy_from_slice(&len.to_be_bytes());

    Ok(data)
}

pub fn validate_mic_token(raw_token: &[u8], key_usage: i32, params: &EncryptionParams) -> Result<()> {
    let token = MicToken::decode(raw_token)?;

    let mut payload = picky_asn1_der::to_vec(&get_mech_list())?;
    payload.extend_from_slice(&token.header());

    // the sub-session key is always preferred over the session key
    let key = if let Some(key) = params.sub_session_key.as_ref() {
        key
    } else if let Some(key) = params.session_key.as_ref() {
        key
    } else {
        return Err(Error::new(ErrorKind::DecryptFailure, "unable to obtain decryption key"));
    };

    let checksum = checksum_sha_aes(key, key_usage, &payload, &params.aes_size().unwrap_or(AesSize::Aes256))?;

    if checksum != token.checksum {
        return Err(Error::new(ErrorKind::MessageAltered, "bad checksum of the mic token"));
    }

    Ok(())
}

pub fn generate_initiator_raw(mut payload: Vec<u8>, seq_number: u64, session_key: &[u8]) -> Result<Vec<u8>> {
    let mut mic_token = MicToken::with_initiator_flags().with_seq_number(seq_number);

    payload.extend_from_slice(&mic_token.header());

    mic_token.set_checksum(checksum_sha_aes(
        session_key,
        INITIATOR_SIGN,
        &payload,
        &AesSize::Aes256,
    )?);

    let mut mic_token_raw = Vec::new();
    mic_token.encode(&mut mic_token_raw)?;

    Ok(mic_token_raw)
}

pub fn unwrap_hostname(hostname: Option<&str>) -> Result<String> {
    if let Some(hostname) = hostname {
        Ok(hostname.into())
    } else {
        Err(Error::new(ErrorKind::InvalidParameter, "The hostname is not provided"))
    }
}

pub fn parse_target_name(target_name: &str) -> Result<(&str, &str)> {
    let divider = target_name.find('/').ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidParameter,
            "Invalid service principal name: missing '/'",
        )
    })?;

    if divider == 0 || divider == target_name.len() - 1 {
        return Err(Error::new(
            ErrorKind::InvalidParameter,
            "Invalid service principal name",
        ));
    }

    let service_name = &target_name[0..divider];
    // `divider + 1` - do not include '/' char
    let service_principal_name = &target_name[(divider + 1)..];

    Ok((service_name, service_principal_name))
}

#[cfg(test)]
mod tests {
    use super::parse_target_name;

    #[test]
    fn parse_valid_target_name() {
        assert_eq!(("EXAMPLE", "p10"), parse_target_name("EXAMPLE/p10").unwrap());
        assert_eq!(("E", "p10"), parse_target_name("E/p10").unwrap());
        assert_eq!(("EXAMPLE", "p"), parse_target_name("EXAMPLE/p").unwrap());
    }

    #[test]
    fn parse_invalid_target_name() {
        assert!(parse_target_name("EXAMPLEp10").is_err());
        assert!(parse_target_name("EXAMPLE/").is_err());
        assert!(parse_target_name("/p10").is_err());
        assert!(parse_target_name("/").is_err());
        assert!(parse_target_name("").is_err());
    }
}
