// tests/net_test.rs
use bakeneko::core::net::ImageCache;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn get_caches_and_serves_from_disk() {
    let hits = Arc::new(AtomicUsize::new(0));
    let hits2 = hits.clone();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for _ in 0..1 { // solo sirve UNA petición
            if let Ok((mut s, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = s.read(&mut buf);
                hits2.fetch_add(1, Ordering::SeqCst);
                let body = b"FAKEIMAGEDATA";
                let resp = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = s.write_all(resp.as_bytes());
                let _ = s.write_all(body);
            }
        }
    });

    temp_env::with_var("XDG_CACHE_HOME", Some("/tmp/net-test-cache"), || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let cache = ImageCache::new();
            let url = format!("http://127.0.0.1:{}/img.jpg", port);

            let p1 = cache.get(&url, &Default::default()).await.unwrap();
            assert!(p1.exists());
            assert_eq!(std::fs::read(&p1).unwrap(), b"FAKEIMAGEDATA");

            // Segunda llamada: desde caché (el server ya no acepta conexiones).
            let p2 = cache.get(&url, &Default::default()).await.unwrap();
            assert_eq!(p1, p2);
            assert_eq!(hits.load(Ordering::SeqCst), 1);
        });
    });
}

#[test]
fn cached_path_is_stable_sha256() {
    temp_env::with_var("XDG_CACHE_HOME", Some("/tmp/net-test-cache2"), || {
        let cache = ImageCache::new();
        let a = cache.cached_path("http://x/1.jpg");
        let b = cache.cached_path("http://x/1.jpg");
        assert_eq!(a, b);
        assert_ne!(a, cache.cached_path("http://x/2.jpg"));
    });
}
