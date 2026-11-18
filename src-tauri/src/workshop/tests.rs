#[cfg(test)]
mod tests {
    use crate::workshop::*;

    fn sample_work(name: &str, bytes: &[u8]) -> Work {
        Work {
            id: "test".into(),
            title: "Test".into(),
            description: String::new(),
            category: "头像".into(),
            version: "1".into(),
            cover: "cover.png".into(),
            archive: "test.zip".into(),
            sha256: "0".repeat(64),
            bytes: 1,
            tags: Vec::new(),
            files: vec![WorkFile {
                name: name.into(),
                sha256: sha256_bytes(bytes),
                bytes: bytes.len() as u64,
                resource: None,
                ..Default::default()
            }],
            cover_data_url: None,
            published_id: None,
            ..Default::default()
        }
    }

    fn sample_zip(path: &Path, name: &str, bytes: &[u8]) {
        let file = fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn archive_validation_rejects_extra_path_and_changed_png() {
        let root = std::env::temp_dir().join(format!(
            "astral-workshop-test-{}-{}",
            std::process::id(),
            unix_now()
        ));
        fs::create_dir_all(&root).unwrap();
        let name = "UT_Hero_ProfilePhoto_112.png";
        let work = sample_work(name, PLACEHOLDER_PNG);
        let archive = root.join("test.zip");
        sample_zip(&archive, name, PLACEHOLDER_PNG);
        assert_eq!(
            unpack_checked(&work, &archive, &root.join("good"))
                .unwrap()
                .len(),
            1
        );
        sample_zip(&archive, "../escape.png", PLACEHOLDER_PNG);
        assert!(unpack_checked(&work, &archive, &root.join("bad-path")).is_err());
        sample_zip(&archive, name, b"broken");
        assert!(unpack_checked(&work, &archive, &root.join("bad-hash")).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn older_work_without_tags_and_localized_resource_tree() {
        let name = "UT_Hero_ProfilePhoto_112_01.png";
        let work = sample_work(name, PLACEHOLDER_PNG);
        let mut json = serde_json::to_value(&work).unwrap();
        json.as_object_mut().unwrap().remove("tags");
        let old: Work = serde_json::from_value(json).unwrap();
        assert!(old.tags.is_empty());
        let resource = resource_for_file(&Library::default(), &old.files[0]).unwrap();
        assert_eq!(resource.category_name, "角色");
        assert_eq!(resource.entity_name, "璐璐");
        assert_eq!(resource.section_name, "小头像");
        assert_eq!(resource.resource_name, "皮肤一");
    }
}
