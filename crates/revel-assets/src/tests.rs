use super::*;

fn project() -> (tempfile::TempDir, Pipeline) {
    let dir = tempfile::tempdir().unwrap();
    let media = dir.path().join("src/media");
    std::fs::create_dir_all(&media).unwrap();
    let img = image::RgbImage::from_fn(1000, 500, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 120])
    });
    img.save(media.join("photo test.jpg")).unwrap();
    std::fs::write(
        media.join("logo.svg"),
        "<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
    )
    .unwrap();
    let p = Pipeline::new(
        dir.path(),
        &[Passthrough {
            from: "src/media".into(),
            to: "media".into(),
        }],
        &[],
    );
    (dir, p)
}

#[test]
fn picture_largeurs_sans_agrandissement() {
    let (_d, p) = project();
    let html = p
        .image(&ImageArgs {
            src: "/media/photo%20test.jpg".into(),
            alt: "Une \"photo\"".into(),
            sizes: "100vw".into(),
            class: "c".into(),
            loading: "eager".into(),
            ..Default::default()
        })
        .unwrap();
    assert!(
        html.starts_with("<picture style=\"display:contents\"><source type=\"image/avif\""),
        "{html}"
    );
    // 480 et 800 ; 1200 et plus sont écartées (pas d'agrandissement).
    assert!(html.contains("-480.avif 480w"));
    assert!(html.contains("-800.avif 800w"));
    assert!(!html.contains("-1000.") && !html.contains("-1200."));
    assert!(html.contains("fetchpriority=\"high\""));
    assert!(html.contains("width=\"800\" height=\"400\""));
    assert!(html.contains("alt=\"Une &quot;photo&quot;\""));
    assert_eq!(p.pending().len(), 6);
}

#[test]
fn svg_et_absent() {
    let (_d, p) = project();
    let html = p
        .image(&ImageArgs {
            src: "/media/logo.svg".into(),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        html,
        "<img src=\"/media/logo.svg\" alt=\"\" loading=\"lazy\" decoding=\"async\">"
    );
    assert!(
        p.image(&ImageArgs {
            src: "/media/absent.jpg".into(),
            ..Default::default()
        })
        .is_none()
    );
}

#[test]
fn production_et_cache() {
    let (d, p) = project();
    let share = p.share_image("/media/photo%20test.jpg", "https://exemple.fr");
    assert!(
        share.starts_with("https://exemple.fr/img/photo test-")
            || share.starts_with("https://exemple.fr/img/photo%20test-"),
        "{share}"
    );
    p.image(&ImageArgs {
        src: "/media/photo test.jpg".into(),
        widths: vec![160, 320],
        ..Default::default()
    });
    let out = d.path().join("_site");
    let s = p.process(&out, &|_, _| {});
    assert_eq!(s.failed, 0, "{:?}", p.take_errors());
    assert_eq!(s.produced, 5); // partage + 2 largeurs x (webp, jpeg)
    let files: Vec<_> = std::fs::read_dir(out.join("img"))
        .unwrap()
        .flatten()
        .collect();
    assert_eq!(files.len(), 5);
    // Second passage : tout vient du cache.
    let p2 = Pipeline::new(
        d.path(),
        &[Passthrough {
            from: "src/media".into(),
            to: "media".into(),
        }],
        &[],
    );
    p2.image(&ImageArgs {
        src: "/media/photo test.jpg".into(),
        widths: vec![160, 320],
        ..Default::default()
    });
    let s2 = p2.process(&out, &|_, _| {});
    assert_eq!((s2.produced, s2.from_cache), (0, 4));
}

#[test]
fn gif_anime_servi_tel_quel() {
    // GIF de 1×1 à une image, puis à deux images.
    let one: &[u8] = &[
        0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 1, 0, 1, 0, 0x80, 0, 0, 0, 0, 0, 255, 255, 255, 0x2C,
        0, 0, 0, 0, 1, 0, 1, 0, 0, 2, 2, 0x44, 0x01, 0, 0x3B,
    ];
    let mut two = one[..one.len() - 1].to_vec();
    two.extend_from_slice(&[
        0x21, 0xF9, 4, 0, 10, 0, 0, 0, 0x2C, 0, 0, 0, 0, 1, 0, 1, 0, 0, 2, 2, 0x44, 0x01, 0, 0x3B,
    ]);
    assert_eq!(super::gif_frames(one), 1);
    assert_eq!(super::gif_frames(&two), 2);
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let media = dir.join("media");
    std::fs::create_dir_all(&media).unwrap();
    std::fs::write(media.join("anim.gif"), &two).unwrap();
    let p = Pipeline::new(
        &dir,
        &[revel_model::Passthrough {
            from: "media".into(),
            to: "media".into(),
        }],
        &[320, 640],
    );
    let html = p
        .image(&ImageArgs {
            src: "/media/anim.gif".into(),
            ..Default::default()
        })
        .unwrap();
    assert!(html.starts_with("<img src=\"/media/anim.gif\""), "{html}");
}
