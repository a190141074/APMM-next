//! 工坊安装的真实隔离文件事务与过期预检验证。
use super::*;

#[test]
#[ignore = "需要独立 ASTRAL_DEV_DATA_DIR，使用 --include-ignored 验收"]
fn workshop_import_only_preserves_game_and_rolls_back_failed_apply() {
    assert!(
        std::env::var("ASTRAL_DEV_DATA_DIR").is_ok(),
        "必须指定隔离数据目录"
    );
    let root = crate::staging::create("workshop-behavior-test").unwrap();
    let game = root.join("game");
    fs::create_dir(&game).unwrap();
    let original_path = game.join("UT_HandCard_10001.png");
    fs::write(&original_path, PLACEHOLDER_PNG).unwrap();
    let mut old = Library::default();
    old.settings.modimg_path = game.to_string_lossy().into();
    let mut work = Work {
        id: uuid::Uuid::new_v4().to_string(),
        version: "1".into(),
        sha256: "0".repeat(64),
        ..Default::default()
    };
    let mut paths = Vec::new();
    for (index, color) in [20, 80].into_iter().enumerate() {
        let path = root.join(format!("file-{index}.png"));
        crate::management_tests::png(&path, color);
        work.files.push(WorkFile {
            name: format!("file-{index}.png"),
            source_name: "同名.png".into(),
            canonical: "UT_HandCard_10001".into(),
            sha256: sha256_file(&path).unwrap(),
            bytes: fs::metadata(&path).unwrap().len(),
            ..Default::default()
        });
        paths.push(path);
    }
    let before = fs::read(&original_path).unwrap();
    let (installed, count) = install::apply_transaction(&work, &paths, &old, &root, &[]).unwrap();
    assert_eq!(count, 2);
    assert_eq!(fs::read(&original_path).unwrap(), before);
    let resource = &installed.heroes["10001"].sections["hand_card"].resources["UT_HandCard_10001"];
    assert_eq!(resource.files.len(), 2);
    assert_ne!(resource.files[0].rel_path, resource.files[1].rel_path);
    assert!(resource.active_hash.is_none());
    let plan = plan::build(work.clone(), &installed).unwrap();
    let mut request = plan::InstallRequest {
        id: work.id.clone(),
        work_fingerprint: plan.work_fingerprint.clone(),
        local_fingerprint: plan.local_fingerprint.clone(),
        apply_files: vec![],
    };
    assert!(plan::validate(&request, &plan).is_ok());
    request.apply_files = work.files.iter().map(|file| file.name.clone()).collect();
    assert!(plan::validate(&request, &plan).is_err());
    request.apply_files = vec![work.files[0].name.clone()];
    assert!(plan::validate(&request, &plan).is_ok());
    crate::management_tests::png(&original_path, 120);
    let changed = plan::build(work.clone(), &installed).unwrap();
    assert!(plan::validate(&request, &changed).is_err());
    // 在库最终提交时制造失败；游戏已经替换后仍必须恢复到提交前字节。
    let current = fs::read(&original_path).unwrap();
    let blocked = library_path().with_extension("json.part");
    fs::create_dir(&blocked).unwrap();
    let result = install::apply_transaction(&work, &paths, &installed, &root, &request.apply_files);
    fs::remove_dir(&blocked).unwrap();
    assert!(result.is_err());
    assert_eq!(fs::read(&original_path).unwrap(), current);
}

#[test]
fn workshop_rejects_fake_png_and_excess_dimensions() {
    assert!(validate_png(b"\x89PNG\r\n\x1a\ninvalid").is_err());
    let mut bytes = PLACEHOLDER_PNG.to_vec();
    bytes[16..20].copy_from_slice(&20000u32.to_be_bytes());
    assert!(validate_png(&bytes).is_err());
}

#[test]
#[ignore = "需要 APMM_INTEROP_FIXTURE，使用 --include-ignored 验收"]
fn workshop_reads_python_signed_v1_v2_over_http() {
    use std::net::TcpListener;
    let fixture: serde_json::Value = serde_json::from_slice(
        &fs::read(std::env::var("APMM_INTEROP_FIXTURE").expect("需要 Python 发布夹具")).unwrap(),
    )
    .unwrap();
    let public = PathBuf::from(fixture["public"].as_str().unwrap());
    std::env::set_var(
        "APMM_WORKSHOP_TEST_PUBLIC_KEY",
        fixture["key"].as_str().unwrap(),
    );
    for legacy in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let public_copy = public.clone();
        let server = std::thread::spawn(move || {
            let until = std::time::Instant::now() + Duration::from_secs(30);
            let mut served = 0;
            while served < if legacy { 3 } else { 1 } {
                assert!(std::time::Instant::now() < until, "目录客户端没有完成请求");
                let (mut stream, _) = match listener.accept() {
                    Ok(value) => value,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    Err(error) => panic!("{error}"),
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = [0; 4096];
                let size = stream.read(&mut request).unwrap();
                let request = String::from_utf8_lossy(&request[..size]);
                let name = request
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .trim_start_matches('/');
                assert!(safe_name(name));
                let missing = legacy && name == "manifest-v2.json";
                let data = if missing {
                    Vec::new()
                } else {
                    fs::read(public_copy.join(name)).unwrap()
                };
                write!(
                    stream,
                    "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    if missing { "404 Not Found" } else { "200 OK" },
                    data.len()
                )
                .unwrap();
                stream.write_all(&data).unwrap();
                served += 1;
            }
        });
        let works = catalog(&format!("http://{address}")).unwrap();
        server.join().unwrap();
        assert_eq!(works.len(), 1);
        let work = &works[0];
        let archive = public.join(&work.archive);
        assert_eq!(sha256_file(&archive).unwrap(), work.sha256);
        let stage = crate::staging::create("python-interop").unwrap();
        let files = unpack_checked(work, &archive, &stage).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(sha256_file(&files[0]).unwrap(), work.files[0].sha256);
    }
    std::env::remove_var("APMM_WORKSHOP_TEST_PUBLIC_KEY");
}
