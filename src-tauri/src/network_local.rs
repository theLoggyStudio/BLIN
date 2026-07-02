use parking_lot::RwLock;
use std::sync::LazyLock;
use std::time::Duration;

static CACHED_IP: LazyLock<RwLock<Option<String>>> = LazyLock::new(|| RwLock::new(None));

const RESOLVE_TIMEOUT: Duration = Duration::from_secs(3);

/// IP locale mise en cache — évite les blocages de `local_ip()` (Win32 GetAdaptersAddresses).
pub fn cached_local_ip() -> String {
    if let Some(ip) = CACHED_IP.read().clone() {
        return ip;
    }
    let ip = resolve_local_ip_with_timeout(RESOLVE_TIMEOUT);
    *CACHED_IP.write() = Some(ip.clone());
    ip
}

fn resolve_local_ip_with_timeout(timeout: Duration) -> String {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let ip = local_ip_address::local_ip()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "127.0.0.1".to_string());
        let _ = tx.send(ip);
    });
    rx.recv_timeout(timeout)
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}

/// Pré-résout l'IP en arrière-plan au démarrage pour que le QR code réponde vite.
pub fn warm_local_ip_cache() {
    std::thread::spawn(|| {
        let _ = cached_local_ip();
    });
}
