use blake2b_simd::Params as Blake2bParams;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::UNIX_EPOCH;

use sha2::{Digest, Sha256};

use crate::types::WalletError;

const ENV_PARAMS_DIR: &str = "LITE_WALLET_ZCASH_PARAMS_DIR";
const ENV_SPEND_SHA256: &str = "LITE_WALLET_SAPLING_SPEND_PARAMS_SHA256";
const ENV_OUTPUT_SHA256: &str = "LITE_WALLET_SAPLING_OUTPUT_PARAMS_SHA256";
const BUILD_SPEND_SHA256: Option<&str> = option_env!("LITE_WALLET_SAPLING_SPEND_PARAMS_SHA256");
const BUILD_OUTPUT_SHA256: Option<&str> = option_env!("LITE_WALLET_SAPLING_OUTPUT_PARAMS_SHA256");

const SAPLING_SPEND_PARAMS_FILE: &str = "sapling-spend.params";
const SAPLING_OUTPUT_PARAMS_FILE: &str = "sapling-output.params";

const SAPLING_SPEND_BLAKE2B_DEFAULT: &str = "8270785a1a0d0bc77196f000ee6d221c9c9894f55307bd9357c3f0105d31ca63991ab91324160d8f53e2bbd3c2633a6eb8bdf5205d822e7f3f73edac51b2b70c";
const SAPLING_OUTPUT_BLAKE2B_DEFAULT: &str = "657e3d38dbb5cb5e7dd2970e8b03d69b4787dd907285b5a7f0790dcc8072f60bf593b32cc2d1c030e00ff5ae64bf84c5c3beb84ddc841d48264b4a171744d028";

const MIN_SPEND_PARAM_SIZE_BYTES: u64 = 40_000_000;
const MIN_OUTPUT_PARAM_SIZE_BYTES: u64 = 3_000_000;
const SAMPLE_BYTES_FOR_PLACEHOLDER_SCAN: usize = 8 * 1024;

pub struct SaplingProvers {
    pub spend: sapling::circuit::SpendParameters,
    pub output: sapling::circuit::OutputParameters,
}

#[derive(Debug, Clone)]
pub struct DlightProverFileDiagnostics {
    pub path: String,
    pub exists: bool,
    pub size_bytes: Option<u64>,
    pub min_size_bytes: u64,
    pub checksum_algorithm: String,
    pub expected_checksum: String,
    pub actual_checksum: Option<String>,
    pub checksum_matches: bool,
    pub placeholder_detected: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DlightProverStatus {
    pub ready: bool,
    pub params_dir: Option<String>,
    pub spend: DlightProverFileDiagnostics,
    pub output: DlightProverFileDiagnostics,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
enum ChecksumAlgorithm {
    Sha256,
    Blake2b512,
}

impl ChecksumAlgorithm {
    fn as_str(self) -> &'static str {
        match self {
            ChecksumAlgorithm::Sha256 => "sha256",
            ChecksumAlgorithm::Blake2b512 => "blake2b-512",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProverStatusFingerprint {
    params_dir: Option<String>,
    spend_path: String,
    output_path: String,
    spend_size: Option<u64>,
    output_size: Option<u64>,
    spend_modified_unix: Option<u64>,
    output_modified_unix: Option<u64>,
    spend_expected_checksum: String,
    output_expected_checksum: String,
}

#[derive(Debug, Clone)]
struct ProverStatusCacheEntry {
    fingerprint: ProverStatusFingerprint,
    status: DlightProverStatus,
}

fn prover_status_cache() -> &'static Mutex<Option<ProverStatusCacheEntry>> {
    static CACHE: OnceLock<Mutex<Option<ProverStatusCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

pub fn get_prover_status() -> DlightProverStatus {
    let params_dir = resolve_params_dir_for_diagnostics();
    let spend_expected = resolve_expected_checksum(
        ENV_SPEND_SHA256,
        BUILD_SPEND_SHA256,
        SAPLING_SPEND_BLAKE2B_DEFAULT,
    );
    let output_expected = resolve_expected_checksum(
        ENV_OUTPUT_SHA256,
        BUILD_OUTPUT_SHA256,
        SAPLING_OUTPUT_BLAKE2B_DEFAULT,
    );
    let fingerprint = build_status_fingerprint(&params_dir, &spend_expected, &output_expected);
    if let Some(cached) = prover_status_cache()
        .lock()
        .ok()
        .and_then(|cache| cache.clone())
        .filter(|entry| entry.fingerprint == fingerprint)
    {
        return cached.status;
    }

    let spend_path = params_dir
        .as_ref()
        .map(|dir| dir.join(SAPLING_SPEND_PARAMS_FILE))
        .unwrap_or_else(|| PathBuf::from(SAPLING_SPEND_PARAMS_FILE));
    let output_path = params_dir
        .as_ref()
        .map(|dir| dir.join(SAPLING_OUTPUT_PARAMS_FILE))
        .unwrap_or_else(|| PathBuf::from(SAPLING_OUTPUT_PARAMS_FILE));

    let spend = inspect_param_file(&spend_path, &spend_expected, MIN_SPEND_PARAM_SIZE_BYTES);
    let output = inspect_param_file(&output_path, &output_expected, MIN_OUTPUT_PARAM_SIZE_BYTES);

    let mut errors = Vec::<String>::new();
    if params_dir.is_none() {
        errors.push(format!(
            "Sapling params directory not found. Set {} or install params into resources/zcash-params.",
            ENV_PARAMS_DIR
        ));
    }
    errors.extend(spend.errors.iter().cloned());
    errors.extend(output.errors.iter().cloned());

    let status = DlightProverStatus {
        ready: errors.is_empty(),
        params_dir: params_dir.map(|dir| dir.display().to_string()),
        spend,
        output,
        errors,
    };

    if let Ok(mut cache) = prover_status_cache().lock() {
        *cache = Some(ProverStatusCacheEntry {
            fingerprint,
            status: status.clone(),
        });
    }

    status
}

pub fn ensure_prover_ready() -> Result<(), WalletError> {
    let status = get_prover_status();
    if status.ready {
        return Ok(());
    }

    eprintln!(
        "[dlight_private][spend_params] prover unavailable: {}",
        status.errors.join(" | ")
    );
    Err(WalletError::DlightProverUnavailable)
}

pub fn load_sapling_provers() -> Result<Arc<SaplingProvers>, WalletError> {
    static CACHE: OnceLock<Mutex<Option<(ProverStatusFingerprint, Arc<SaplingProvers>)>>> =
        OnceLock::new();
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| WalletError::DlightProverUnavailable)?;
    let status = get_prover_status();
    if !status.ready {
        return Err(WalletError::DlightProverUnavailable);
    }
    let fingerprint = build_status_fingerprint(
        &status.params_dir.as_ref().map(PathBuf::from),
        &status.spend.expected_checksum,
        &status.output.expected_checksum,
    );
    if let Some((cached_fingerprint, provers)) = cache.as_ref() {
        if *cached_fingerprint == fingerprint {
            return Ok(provers.clone());
        }
    }

    let params_dir = locate_params_dir().ok_or(WalletError::DlightProverUnavailable)?;
    let spend_path = params_dir.join(SAPLING_SPEND_PARAMS_FILE);
    let output_path = params_dir.join(SAPLING_OUTPUT_PARAMS_FILE);

    // Parse the exact bytes that pass checksum verification, so a file change
    // between diagnostics and loading cannot replace the proving material.
    let spend_bytes = read_verified_params(&spend_path, &status.spend.expected_checksum)?;
    let output_bytes = read_verified_params(&output_path, &status.output.expected_checksum)?;
    let spend = sapling::circuit::SpendParameters::read(spend_bytes.as_slice(), false)
        .map_err(|_| WalletError::DlightProverUnavailable)?;
    let output = sapling::circuit::OutputParameters::read(output_bytes.as_slice(), false)
        .map_err(|_| WalletError::DlightProverUnavailable)?;

    let provers = Arc::new(SaplingProvers { spend, output });
    *cache = Some((fingerprint, provers.clone()));
    Ok(provers)
}

fn read_verified_params(path: &Path, expected: &str) -> Result<Vec<u8>, WalletError> {
    let bytes = std::fs::read(path).map_err(|_| WalletError::DlightProverUnavailable)?;
    let actual =
        match resolve_checksum_algorithm(expected).ok_or(WalletError::DlightProverUnavailable)? {
            ChecksumAlgorithm::Sha256 => hex::encode(Sha256::digest(&bytes)),
            ChecksumAlgorithm::Blake2b512 => Blake2bParams::new()
                .hash_length(64)
                .hash(&bytes)
                .to_hex()
                .to_string(),
        };
    if actual != expected {
        return Err(WalletError::DlightProverUnavailable);
    }
    Ok(bytes)
}

fn resolve_params_dir_for_diagnostics() -> Option<PathBuf> {
    if let Ok(raw) = std::env::var(ENV_PARAMS_DIR) {
        let candidate = PathBuf::from(raw.trim());
        if !candidate.as_os_str().is_empty() {
            return Some(candidate);
        }
    }

    locate_params_dir().or_else(|| candidate_params_dirs().into_iter().next())
}

fn locate_params_dir() -> Option<PathBuf> {
    if let Ok(raw) = std::env::var(ENV_PARAMS_DIR) {
        let candidate = PathBuf::from(raw.trim());
        if has_required_files(&candidate) {
            return Some(candidate);
        }
    }

    candidate_params_dirs()
        .into_iter()
        .find(|candidate| has_required_files(candidate))
}

fn candidate_params_dirs() -> Vec<PathBuf> {
    let mut candidates = Vec::<PathBuf>::new();
    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join("src-tauri/resources/zcash-params"));
        candidates.push(current_dir.join("resources/zcash-params"));
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(exe_dir.join("resources/zcash-params"));
            candidates.push(exe_dir.join("../resources/zcash-params"));
            candidates.push(exe_dir.join("../Resources/zcash-params"));
        }
    }

    candidates
}

fn has_required_files(dir: &Path) -> bool {
    dir.join(SAPLING_SPEND_PARAMS_FILE).exists() && dir.join(SAPLING_OUTPUT_PARAMS_FILE).exists()
}

fn resolve_expected_checksum(
    runtime_var: &str,
    build_value: Option<&str>,
    default_value: &str,
) -> String {
    if let Ok(value) = std::env::var(runtime_var) {
        let normalized = value.trim().to_ascii_lowercase();
        if !normalized.is_empty() {
            return normalized;
        }
    }

    if let Some(value) = build_value {
        let normalized = value.trim().to_ascii_lowercase();
        if !normalized.is_empty() {
            return normalized;
        }
    }

    default_value.to_string()
}

fn build_status_fingerprint(
    params_dir: &Option<PathBuf>,
    spend_expected_checksum: &str,
    output_expected_checksum: &str,
) -> ProverStatusFingerprint {
    let spend_path = params_dir
        .as_ref()
        .map(|dir| dir.join(SAPLING_SPEND_PARAMS_FILE))
        .unwrap_or_else(|| PathBuf::from(SAPLING_SPEND_PARAMS_FILE));
    let output_path = params_dir
        .as_ref()
        .map(|dir| dir.join(SAPLING_OUTPUT_PARAMS_FILE))
        .unwrap_or_else(|| PathBuf::from(SAPLING_OUTPUT_PARAMS_FILE));

    let spend_metadata = std::fs::metadata(&spend_path).ok();
    let output_metadata = std::fs::metadata(&output_path).ok();

    ProverStatusFingerprint {
        params_dir: params_dir.as_ref().map(|value| value.display().to_string()),
        spend_path: spend_path.display().to_string(),
        output_path: output_path.display().to_string(),
        spend_size: spend_metadata.as_ref().map(|meta| meta.len()),
        output_size: output_metadata.as_ref().map(|meta| meta.len()),
        spend_modified_unix: spend_metadata
            .as_ref()
            .and_then(metadata_modified_unix_secs),
        output_modified_unix: output_metadata
            .as_ref()
            .and_then(metadata_modified_unix_secs),
        spend_expected_checksum: spend_expected_checksum.to_ascii_lowercase(),
        output_expected_checksum: output_expected_checksum.to_ascii_lowercase(),
    }
}

fn metadata_modified_unix_secs(metadata: &std::fs::Metadata) -> Option<u64> {
    let modified = metadata.modified().ok()?;
    let duration = modified.duration_since(UNIX_EPOCH).ok()?;
    Some(duration.as_secs())
}

fn inspect_param_file(
    path: &Path,
    expected_checksum: &str,
    min_size_bytes: u64,
) -> DlightProverFileDiagnostics {
    let mut errors = Vec::<String>::new();

    let metadata = std::fs::metadata(path).ok();
    let exists = metadata.is_some();
    let size_bytes = metadata.as_ref().map(|meta| meta.len());

    if !exists {
        errors.push(format!(
            "Required params file is missing: {}",
            path.display()
        ));
    }

    let checksum_algorithm = resolve_checksum_algorithm(expected_checksum);
    if checksum_algorithm.is_none() {
        errors.push(format!(
            "Unsupported checksum format for {}. Expected 64-char sha256 or 128-char blake2b-512.",
            path.display()
        ));
    }

    let mut placeholder_detected = false;
    if let Some(size) = size_bytes {
        if size < min_size_bytes {
            placeholder_detected = true;
            errors.push(format!(
                "Params file appears to be a placeholder (size {} bytes, expected at least {}).",
                size, min_size_bytes
            ));
        }

        if file_contains_placeholder_marker(path).unwrap_or(false) {
            placeholder_detected = true;
            errors.push(format!(
                "Params file appears to contain placeholder marker text: {}",
                path.display()
            ));
        }
    }

    let expected_checksum = expected_checksum.trim().to_ascii_lowercase();
    let actual_checksum = if exists && checksum_algorithm.is_some() {
        checksum_algorithm
            .and_then(|algorithm| compute_checksum(path, algorithm).ok())
            .map(|value| value.to_ascii_lowercase())
    } else {
        None
    };

    let checksum_matches = actual_checksum
        .as_ref()
        .map(|actual| actual == &expected_checksum)
        .unwrap_or(false);

    if exists && checksum_algorithm.is_some() && !checksum_matches {
        errors.push(format!(
            "Checksum mismatch for {} (expected {}, got {}).",
            path.display(),
            expected_checksum,
            actual_checksum
                .clone()
                .unwrap_or_else(|| "unavailable".to_string())
        ));
    }

    DlightProverFileDiagnostics {
        path: path.display().to_string(),
        exists,
        size_bytes,
        min_size_bytes,
        checksum_algorithm: checksum_algorithm
            .map(ChecksumAlgorithm::as_str)
            .unwrap_or("unknown")
            .to_string(),
        expected_checksum,
        actual_checksum,
        checksum_matches,
        placeholder_detected,
        errors,
    }
}

fn resolve_checksum_algorithm(expected: &str) -> Option<ChecksumAlgorithm> {
    let normalized = expected.trim().to_ascii_lowercase();
    if normalized.is_empty() || !normalized.chars().all(|char| char.is_ascii_hexdigit()) {
        return None;
    }

    match normalized.len() {
        64 => Some(ChecksumAlgorithm::Sha256),
        128 => Some(ChecksumAlgorithm::Blake2b512),
        _ => None,
    }
}

fn file_contains_placeholder_marker(path: &Path) -> Result<bool, WalletError> {
    let mut file = File::open(path).map_err(|_| WalletError::OperationFailed)?;
    let mut buffer = vec![0u8; SAMPLE_BYTES_FOR_PLACEHOLDER_SCAN];
    let read = file
        .read(&mut buffer)
        .map_err(|_| WalletError::OperationFailed)?;
    if read == 0 {
        return Ok(false);
    }

    let text = String::from_utf8_lossy(&buffer[..read]).to_ascii_lowercase();
    Ok(text.contains("placeholder") || text.contains("replace this file"))
}

fn compute_checksum(path: &Path, algorithm: ChecksumAlgorithm) -> Result<String, WalletError> {
    match algorithm {
        ChecksumAlgorithm::Sha256 => compute_sha256(path),
        ChecksumAlgorithm::Blake2b512 => compute_blake2b512(path),
    }
}

fn compute_sha256(path: &Path) -> Result<String, WalletError> {
    let mut file = File::open(path).map_err(|_| WalletError::OperationFailed)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| WalletError::OperationFailed)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

fn compute_blake2b512(path: &Path) -> Result<String, WalletError> {
    let mut file = File::open(path).map_err(|_| WalletError::OperationFailed)?;
    let mut hasher = Blake2bParams::new().hash_length(64).to_state();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| WalletError::OperationFailed)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::{inspect_param_file, resolve_checksum_algorithm, ChecksumAlgorithm};

    #[test]
    fn checksum_algorithm_supports_sha256_and_blake2b() {
        assert!(matches!(
            resolve_checksum_algorithm(
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            ),
            Some(ChecksumAlgorithm::Sha256)
        ));

        assert!(matches!(
            resolve_checksum_algorithm(
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            ),
            Some(ChecksumAlgorithm::Blake2b512)
        ));
    }

    #[test]
    fn inspect_rejects_placeholder_sized_file() {
        let temp_dir = std::env::temp_dir().join(format!(
            "lite-wallet-spend-params-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&temp_dir).expect("create temp dir");
        let file_path = temp_dir.join("sapling-spend.params");
        std::fs::write(&file_path, b"placeholder").expect("write file");

        let diagnostics = inspect_param_file(
            &file_path,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            1_024,
        );

        assert!(diagnostics.placeholder_detected);
        assert!(!diagnostics.errors.is_empty());

        let _ = std::fs::remove_file(file_path);
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn inspect_accepts_matching_sha256_when_size_is_sane() {
        let temp_dir = std::env::temp_dir().join(format!(
            "lite-wallet-spend-params-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&temp_dir).expect("create temp dir");
        let file_path = temp_dir.join("sapling-output.params");
        std::fs::write(&file_path, b"abcd").expect("write file");

        let diagnostics = inspect_param_file(
            &file_path,
            "88d4266fd4e6338d13b845fcf289579d209c897823b9217da3e161936f031589",
            4,
        );

        assert!(diagnostics.exists);
        assert!(diagnostics.checksum_matches);
        assert!(diagnostics.errors.is_empty());

        let _ = std::fs::remove_file(file_path);
        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
