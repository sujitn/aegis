//! Certificate Authority management for MITM proxy.
//!
//! Generates and manages the root CA certificate used to sign per-domain
//! certificates on the fly.

use std::fs;
use std::path::{Path, PathBuf};

use hudsucker::certificate_authority::RcgenAuthority;
use hudsucker::rcgen::{CertificateParams, Issuer, KeyPair};
use hudsucker::rustls::crypto::aws_lc_rs::default_provider;

pub use crate::error::CaManagerError;

/// CA certificate and key file names.
const CA_CERT_FILENAME: &str = "aegis-ca.crt";
const CA_KEY_FILENAME: &str = "aegis-ca.key";

/// Manages the root CA certificate for the MITM proxy.
#[derive(Debug, Clone)]
pub struct CaManager {
    /// Path to the CA directory.
    ca_dir: PathBuf,
}

impl CaManager {
    /// Creates a new CA manager with the given directory.
    pub fn new(ca_dir: impl AsRef<Path>) -> Self {
        Self {
            ca_dir: ca_dir.as_ref().to_path_buf(),
        }
    }

    /// Creates a CA manager using the default Aegis data directory.
    pub fn with_default_dir() -> Result<Self, CaManagerError> {
        let project_dirs = directories::ProjectDirs::from("com", "aegis", "Aegis")
            .ok_or_else(|| CaManagerError::Generation("Failed to get project dirs".into()))?;

        let ca_dir = project_dirs.data_dir().join("ca");
        Ok(Self::new(ca_dir))
    }

    /// Returns the path to the CA certificate file.
    pub fn cert_path(&self) -> PathBuf {
        self.ca_dir.join(CA_CERT_FILENAME)
    }

    /// Returns the path to the CA private key file.
    pub fn key_path(&self) -> PathBuf {
        self.ca_dir.join(CA_KEY_FILENAME)
    }

    /// Checks if the CA certificate exists.
    pub fn ca_exists(&self) -> bool {
        self.cert_path().exists() && self.key_path().exists()
    }

    /// Ensures the CA certificate exists, generating it if necessary.
    ///
    /// Returns the hudsucker RcgenAuthority ready for use with the proxy.
    pub fn ensure_ca(&self) -> Result<RcgenAuthority, CaManagerError> {
        if !self.ca_exists() {
            self.generate_ca()?;
        }
        self.load_authority()
    }

    /// Generates a new root CA certificate and key.
    pub fn generate_ca(&self) -> Result<(), CaManagerError> {
        // Ensure directory exists
        fs::create_dir_all(&self.ca_dir)?;

        // Generate key pair
        let key_pair =
            KeyPair::generate().map_err(|e| CaManagerError::Generation(e.to_string()))?;

        // Build certificate parameters for a CA
        let mut params = CertificateParams::new(vec!["Aegis Root CA".to_string()])
            .map_err(|e| CaManagerError::Generation(e.to_string()))?;

        // Set as CA
        params.is_ca =
            hudsucker::rcgen::IsCa::Ca(hudsucker::rcgen::BasicConstraints::Unconstrained);

        // Set key usages for CA
        params.key_usages = vec![
            hudsucker::rcgen::KeyUsagePurpose::KeyCertSign,
            hudsucker::rcgen::KeyUsagePurpose::CrlSign,
            hudsucker::rcgen::KeyUsagePurpose::DigitalSignature,
        ];

        // Extended key usage
        params.extended_key_usages = vec![
            hudsucker::rcgen::ExtendedKeyUsagePurpose::ServerAuth,
            hudsucker::rcgen::ExtendedKeyUsagePurpose::ClientAuth,
        ];

        // Generate self-signed certificate
        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| CaManagerError::Generation(e.to_string()))?;

        // Write certificate (PEM format)
        let cert_pem = cert.pem();
        fs::write(self.cert_path(), &cert_pem).map_err(|e| CaManagerError::Write(e.to_string()))?;

        // Write private key (PEM format)
        let key_pem = key_pair.serialize_pem();
        fs::write(self.key_path(), &key_pem).map_err(|e| CaManagerError::Write(e.to_string()))?;

        tracing::info!("Generated new CA certificate at {:?}", self.cert_path());

        Ok(())
    }

    /// Loads the CA certificate and creates a hudsucker authority.
    pub fn load_authority(&self) -> Result<RcgenAuthority, CaManagerError> {
        // Read certificate and key PEM files
        let cert_pem = fs::read_to_string(self.cert_path())?;
        let key_pem = fs::read_to_string(self.key_path())?;

        // Parse the key pair from PEM
        let key_pair =
            KeyPair::from_pem(&key_pem).map_err(|e| CaManagerError::Parse(e.to_string()))?;

        // Create issuer from CA cert PEM and key pair
        let issuer = Issuer::from_ca_cert_pem(&cert_pem, key_pair)
            .map_err(|e| CaManagerError::Parse(e.to_string()))?;

        // Create rcgen authority with crypto provider
        let authority = RcgenAuthority::new(issuer, 1000, default_provider());

        Ok(authority)
    }

    /// Reads the CA certificate as DER bytes (for installation instructions).
    pub fn read_cert_der(&self) -> Result<Vec<u8>, CaManagerError> {
        // Load the key and regenerate the cert
        let key_pem = fs::read_to_string(self.key_path())?;
        let key_pair =
            KeyPair::from_pem(&key_pem).map_err(|e| CaManagerError::Parse(e.to_string()))?;

        let mut params = CertificateParams::new(vec!["Aegis Root CA".to_string()])
            .map_err(|e| CaManagerError::Parse(e.to_string()))?;

        params.is_ca =
            hudsucker::rcgen::IsCa::Ca(hudsucker::rcgen::BasicConstraints::Unconstrained);
        params.key_usages = vec![
            hudsucker::rcgen::KeyUsagePurpose::KeyCertSign,
            hudsucker::rcgen::KeyUsagePurpose::CrlSign,
            hudsucker::rcgen::KeyUsagePurpose::DigitalSignature,
        ];
        params.extended_key_usages = vec![
            hudsucker::rcgen::ExtendedKeyUsagePurpose::ServerAuth,
            hudsucker::rcgen::ExtendedKeyUsagePurpose::ClientAuth,
        ];

        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| CaManagerError::Parse(e.to_string()))?;

        Ok(cert.der().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn ca_manager_new() {
        let manager = CaManager::new("/tmp/test-ca");
        assert_eq!(manager.ca_dir, PathBuf::from("/tmp/test-ca"));
    }

    #[test]
    fn ca_manager_paths() {
        let manager = CaManager::new("/tmp/test-ca");
        assert_eq!(
            manager.cert_path(),
            PathBuf::from("/tmp/test-ca/aegis-ca.crt")
        );
        assert_eq!(
            manager.key_path(),
            PathBuf::from("/tmp/test-ca/aegis-ca.key")
        );
    }

    #[test]
    fn ca_manager_not_exists_initially() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CaManager::new(temp_dir.path().join("ca"));
        assert!(!manager.ca_exists());
    }

    #[test]
    fn ca_manager_generate_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CaManager::new(temp_dir.path().join("ca"));

        // Generate CA
        manager.generate_ca().unwrap();
        assert!(manager.ca_exists());

        // Verify files exist
        assert!(manager.cert_path().exists());
        assert!(manager.key_path().exists());

        // Load authority
        let authority = manager.load_authority();
        assert!(authority.is_ok());
    }

    #[test]
    fn ca_manager_ensure_ca_generates_if_missing() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CaManager::new(temp_dir.path().join("ca"));

        assert!(!manager.ca_exists());

        // ensure_ca should generate
        let authority = manager.ensure_ca();
        assert!(authority.is_ok());
        assert!(manager.ca_exists());
    }

    #[test]
    fn ca_manager_ensure_ca_loads_if_exists() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CaManager::new(temp_dir.path().join("ca"));

        // Generate first
        manager.generate_ca().unwrap();

        // ensure_ca should just load
        let authority = manager.ensure_ca();
        assert!(authority.is_ok());
    }

    #[test]
    fn ca_manager_read_cert_der() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CaManager::new(temp_dir.path().join("ca"));

        manager.generate_ca().unwrap();

        let der = manager.read_cert_der();
        assert!(der.is_ok());
        assert!(!der.unwrap().is_empty());
    }
}
