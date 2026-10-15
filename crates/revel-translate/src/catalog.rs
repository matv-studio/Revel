//! Catalogue des langues de traduction (D-36) : modèles Marian OPUS-MT de l'université
//! d'Helsinki (licence Apache-2.0, <https://huggingface.co/Helsinki-NLP>), une langue = deux
//! modèles, depuis et vers l'anglais (langue intermédiaire entre deux autres langues).
//!
//! Révisions figées et empreintes SHA-256 de chaque fichier : un téléchargement ne peut pas être
//! remplacé à l'insu de Revel. Sans poids `model.safetensors` dans la branche principale, révision
//! de la conversion proposée par le robot de Hugging Face (`refs/pr/N`), ou poids Flax.
//! Table produite par `tools/models-catalog.py` (ne pas modifier à la main).

/// Fichier d'un modèle à télécharger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteFile {
    pub name: &'static str,
    pub size: u64,
    pub sha256: &'static str,
}

/// Modèle d'origine (dépôt `Helsinki-NLP/<name>` à la révision `rev`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Source {
    pub name: &'static str,
    pub rev: &'static str,
    pub files: &'static [RemoteFile],
}

/// Langue : modèles vers l'anglais et depuis l'anglais.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pack {
    pub code: &'static str,
    pub models: [Source; 2],
}

/// Langue intermédiaire (comprise dans chaque langue du catalogue).
pub const HUB: &str = "en";

/// Langue du catalogue.
pub fn pack(code: &str) -> Option<&'static Pack> {
    PACKS.iter().find(|p| p.code == code)
}

impl Pack {
    /// Taille à télécharger, en octets.
    pub fn download_size(&self) -> u64 {
        self.models
            .iter()
            .flat_map(|m| m.files.iter())
            .map(|f| f.size)
            .sum()
    }

    /// Taille une fois préparé (poids en 16 bits), en octets (estimation).
    pub fn disk_size(&self) -> u64 {
        self.models
            .iter()
            .flat_map(|m| m.files.iter())
            .map(|f| {
                if is_weights(f.name) {
                    f.size / 2
                } else {
                    f.size
                }
            })
            .sum()
    }
}

/// Fichier de poids (converti en 16 bits à la préparation).
pub fn is_weights(name: &str) -> bool {
    name == "model.safetensors" || name == "flax_model.msgpack"
}

/// Jeton de langue placé en tête du texte pour les modèles qui produisent plusieurs langues
/// (`>>cmn_Hans<<` : chinois simplifié).
pub fn target_token(model: &str, to: &str) -> Option<&'static str> {
    let to = to.to_ascii_lowercase();
    match model {
        "opus-mt-en-zh" => Some(
            if ["hant", "tw", "hk", "mo"]
                .iter()
                .any(|v| to.split(['-', '_']).any(|s| s == *v))
            {
                ">>cmn_Hant<<"
            } else {
                ">>cmn_Hans<<"
            },
        ),
        "opus-mt-en-ar" => Some(">>ara<<"),
        "opus-mt-en-bg" => Some(">>bul<<"),
        "opus-mt-en-vi" => Some(">>vie<<"),
        _ => None,
    }
}

/// Langues téléchargeables.
pub static PACKS: &[Pack] = &[
    Pack {
        code: "fr",
        models: [
            Source {
                name: "opus-mt-fr-en",
                rev: "c4aed37b318c763fd177aa449b44e3b783cc6c02",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1416,
                        sha256: "b3be13d046d9899d7aab8cf4ed624d9a79f5776038ba793f6b4d2ce3e02192f7",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 802397,
                        sha256: "78d0e717c77053f1c4b856d8661d9cb87c64f083a35418c087b9146300e4f585",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1339166,
                        sha256: "945c604346ce15ce4aff9001001e7f925e336d942c4087017f191871162cbdc4",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 300803608,
                        sha256: "6e3837f34b903802c3d0d670362b997cee6e87584a1108eb3fa89e4625e4424a",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-fr",
                rev: "dd7f6540a7a48a7f4db59e5c0b9c42c8eea67f18",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1416,
                        sha256: "df92c371911b45282caac6101fe7e7f5d5e9af7f9a99356067ffe306278ce72b",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 778395,
                        sha256: "173e9f493a668fe396d599e28d414a201193094e6ffd7a4678e5aab0f6d3d838",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1339166,
                        sha256: "945c604346ce15ce4aff9001001e7f925e336d942c4087017f191871162cbdc4",
                    },
                    RemoteFile {
                        name: "flax_model.msgpack",
                        size: 298685650,
                        sha256: "7b7ebb1b554e24be9f05afbe805ad155e972695624f261c2328f420d880373bf",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "de",
        models: [
            Source {
                name: "opus-mt-de-en",
                rev: "8be33f00bcaf451dc5ed05fdd94282649696b8e7",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "89368ef76ea89581025cdf605caac75b8a22af2c1a90ec57c8a5001f10537eeb",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 796845,
                        sha256: "bbd1f495eea99c8e21ae086d9146e0fa7b096c3dfdd9ba07ab8b631889df5c9b",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1273232,
                        sha256: "0d70d89fee4a8b4ef99a56d712163baadcabd5600a597f71515547ee70306329",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 297903668,
                        sha256: "5b1a66c79f6e871eb01b8818819e29bbdb6a25139af210b97b6efbf862493b4d",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-de",
                rev: "6183067f769a302e3861815543b9f312c71b0ca4",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1335,
                        sha256: "41c7890e6f6c1f72c5c626569cfe739a8362080940edbb1747f25173f541653b",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 768489,
                        sha256: "678f2a1177d8389f67b66299762dcc4fc567e89b07e212ba91b0c56daecf47ce",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1273232,
                        sha256: "0d70d89fee4a8b4ef99a56d712163baadcabd5600a597f71515547ee70306329",
                    },
                    RemoteFile {
                        name: "flax_model.msgpack",
                        size: 295786174,
                        sha256: "1fcf707093bb204211b4e7f8b487896fc65623fd290565992b2105f759539520",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "es",
        models: [
            Source {
                name: "opus-mt-es-en",
                rev: "d7a4e7e03d08f8aa6ed2948f8f52d0285bed7cce",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1438,
                        sha256: "374443eacf8986c21386503e2fcd52eae6952ac09c87fd36695f5c12259a4cd1",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 825924,
                        sha256: "e236ee6d866b635c0142114f8647f39831f9d92534aa2aad75c942f6a78ad0e3",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1590040,
                        sha256: "257f346d7a6b2ecceafcca8ba05648ce2fd68dfaf105fb0e913dca7198f3f6d5",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 312062468,
                        sha256: "be17813aa5fbfa8a0f3c78e441c0461a76c68c9e63461bbbfe3ee12296fad3f1",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-es",
                rev: "5bc4493d463cf000c1f0b50f8d56886a392ed4ab",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1473,
                        sha256: "666ad3c2943674b6a789e311d221e44ec23e27f1bd9727930754bf773ec8e464",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 801636,
                        sha256: "4dd547c24816a335e7b0b2e63376a8f1b3cbfc671eda5ab808dd44fdadaa8791",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1590040,
                        sha256: "257f346d7a6b2ecceafcca8ba05648ce2fd68dfaf105fb0e913dca7198f3f6d5",
                    },
                    RemoteFile {
                        name: "flax_model.msgpack",
                        size: 309944974,
                        sha256: "14b5e0865da9b6764e18b637e7c7e34b04be927dc41a1ba97bff23962c2f5857",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "it",
        models: [
            Source {
                name: "opus-mt-it-en",
                rev: "3a1b763a296ef5e4dbc9c6f12c014447ba61c88e",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "579f74b8a178cb5d006a521e497d2de643185160d45da2df7cdf2cda65ea2861",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 813709,
                        sha256: "1bd307e2756991d470e4b40172147f7889b46a323b1a9a2150e66274825b1f21",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 2369833,
                        sha256: "f4ea61764cde265f25c2824bfe5b945685526ede81c5f2f76715a5652a63ccdb",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 343618124,
                        sha256: "a6e1dd4180f8aeec864ea5d79678025fa013e93df935d46159ebd6eb164782c1",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-it",
                rev: "4106a035fb5cb94073793de781668e47c13015cb",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "e0500492a3168b45a79bb6e3f0373d0190a9e3730e83d1316628b31f1b5a50c0",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 789468,
                        sha256: "2efff2c37e51842c2817a6770280979b5dd803cebbe2928e576d42a888f89537",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 2351575,
                        sha256: "eb0d25eea013793e15cd574a698105dc147a5d4ba522ba5ddb8fb7554c51d37b",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 342912236,
                        sha256: "7012b826195f82a68efc878b39e763d2fbf8ebb485811375e9d9632899eba7d0",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "nl",
        models: [
            Source {
                name: "opus-mt-nl-en",
                rev: "aba87d1c1a6e4be1563fd04c819f27c70f442ebf",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "d0e83dd5b75e1680ead94e4166b386bf901320455f0494120cbf629044fef1ba",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 813866,
                        sha256: "f538d23d4b30641cd97c608d2a83890d503aa1286a5560f19ef949404a786e94",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1660216,
                        sha256: "52ef60fa185ed5a6a6e75a4ea45de99582f1c1493df1c072a784489697407b2b",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 316221872,
                        sha256: "d0ea062014619242edc59e3f21d5d3c93cb9c8c7c707bf7d2932c57f90399e99",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-nl",
                rev: "e80d73cca9576643fb5a8fa94e73d45ea38b4ea4",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "d64feaab43c91ef67f7c743ba539f520ce554abcca859f67a00b1346e01bb48c",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 789525,
                        sha256: "d05781fde84b8793191528a7a176e1b17f96265388f602379ee6ae3139f59141",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1660216,
                        sha256: "52ef60fa185ed5a6a6e75a4ea45de99582f1c1493df1c072a784489697407b2b",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 316221872,
                        sha256: "1e227db75d9134498000e1e91c9c8f6f6347d586d5fa2b7b08ea2c4e630c24db",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "sv",
        models: [
            Source {
                name: "opus-mt-sv-en",
                rev: "8f12539d775fc72dd0cb156271db4f939c8e5e18",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "d47bbf76b7c3b746f3665a75f5da7f4e906cde98ef2c41f16dae46e00e0d2342",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 815294,
                        sha256: "845278915ae2a708063110903d08c1e75ebb36607ef16ec9530955e1e95e62b8",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1291486,
                        sha256: "24e474ba6c733601b3538b4ee05d9db7275caf45a2996c0cce9fb67c70b4a95b",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 294482984,
                        sha256: "4e81e33e559aefc902ddebcf955a20ae966da2429ecc2a9b4740563f207f6dea",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-sv",
                rev: "75a9b8e71abcb314441c277d8385e81554dc2f78",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "02c519c7bad6da35ddab8c0462826387dde4b2109d4f40b066dd4d008bc96259",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 790085,
                        sha256: "70d69a2c08a7a2c18e4529c8ff62886aad2f8fd0e94fedeafe4bafc47a5f0d69",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1291486,
                        sha256: "24e474ba6c733601b3538b4ee05d9db7275caf45a2996c0cce9fb67c70b4a95b",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 294482984,
                        sha256: "1f10f4b66c86e4d98cdfa01e842ed2735312a6a99da08978b64e579e875fb0cb",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "da",
        models: [
            Source {
                name: "opus-mt-da-en",
                rev: "4f281343f8c8db3e1363cc30bcc5e0b8cfa71dd3",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "f0ab60be8c9b8f3c09309f4c51fd28ac83568c9bc00b36966f5983d35c1a7e4d",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 819654,
                        sha256: "c97d12d35fe411947392d927f0be14985e2ea0e5cf258164db7f37780563ca15",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1382006,
                        sha256: "dda6d56f58cb917811730faaa9f81586ad9604b71b55890cdd3d99fb8376032d",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 299604776,
                        sha256: "a8effdcf176755cc436f107719db0b082d9156c9ae54e61dd426f2f3b4f36243",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-da",
                rev: "8df3c9ed784d5b7c54f1547f1f8d472a43a24667",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "926209674c35ccc61892ce2c026af4411233f534578add5133842ca0e9d7d943",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 788462,
                        sha256: "08d1d21ea3f6f80f06484db753c1a60238b2acfe4aa0fafd496be869857f697f",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1382006,
                        sha256: "dda6d56f58cb917811730faaa9f81586ad9604b71b55890cdd3d99fb8376032d",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 299604776,
                        sha256: "af93346239b6935d940ea0d00aff951915d7732027644f1cf9cb5f17e1adf5a4",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "fi",
        models: [
            Source {
                name: "opus-mt-fi-en",
                rev: "97f67c6bcfdf477948cfe0330f663e3d7a46e7ce",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1394,
                        sha256: "da465a0d1d835acb5a8fe4b9b91fac713c0ea4541d1acc55e7e6fe2387c1fdeb",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 831865,
                        sha256: "82fcb74f79c5f5b5bf0fe73f7dfb6a646efa50ffb4b2a0cd564a1193c78a032e",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1377921,
                        sha256: "d57a7bc71f655a6d466feb7c5f547fdec978fce756a1b34b864197aa4fa168de",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 301100684,
                        sha256: "2f8b591f7c7543ecf557653e8b90900478fc134adc88df62404b30cf95958c28",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-fi",
                rev: "51a3df99887e8df44e9235f30d52d248d46f0f0c",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1361,
                        sha256: "f574dd86fd2017768d1e24f9028b678a6bf838d34cf9066746e5786a698dedb0",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 803110,
                        sha256: "412535c7eb38271082f86d9574881af46057dddaf17be876da8021557104a488",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1592256,
                        sha256: "e54a6bc7501989cb88933482a43a51daf685360f824e9d26e23967e9252a688f",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 312062468,
                        sha256: "2a43d7e0416faf10acb80a106d470f02791f0d6be7a36bcc011cd67943fd07d2",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "cs",
        models: [
            Source {
                name: "opus-mt-cs-en",
                rev: "8e6900031545fd8e7f80f5000da8a282bc93044a",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "98b9d71aa0e0788e0018eefcf02ccabf043f9ce9c91209bad7eccf0e1e4f5bef",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 829548,
                        sha256: "a2e6d98cdd54d80ad404ad493a7755e723c8d619e998840eba682e3e3fe25e33",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1596450,
                        sha256: "cd35c38843ebe40679f99e6922d8c844326e0bfe65066b52c735bc68c860180d",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 306948884,
                        sha256: "28fcc2e7d7cfcea474fe35fe9ac0471dcfa165d5716db5189a19b8035c6e48aa",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-cs",
                rev: "d32de408381844b7b4b2465bf702198d64643b6c",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "18d6ce2ed089abb39af5b95438e24bda8664b6ac17be644de806beda278792ab",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 788721,
                        sha256: "8cb772d24cbd559a9e0c5dfd4726e1201b07497af59189f107dd812bff1909e7",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1596450,
                        sha256: "cd35c38843ebe40679f99e6922d8c844326e0bfe65066b52c735bc68c860180d",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 306948884,
                        sha256: "79f82c7a1228c0e56cd014af4f8712405908df1bd9099e0d873f0bf3b6374204",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "sk",
        models: [
            Source {
                name: "opus-mt-sk-en",
                rev: "2a2c855e3df8443d863f046ab41b16d7221b918e",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "171f6a6d80a0bd6145a7cae541c7073d0316948ca87cac4a9293fe7ae215b80c",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 829895,
                        sha256: "532600fd2cbe88ecce896c892ffc7b48f21ca8d821cfd479fdb4b12158c73784",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1461131,
                        sha256: "f933c1374027186cbde2ce24517e307b27751c3e3fe910262653656037ff2326",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 301851716,
                        sha256: "fbc14b2da3b4d57886e96fddab861120e5053b7ef9dc0b0019943c23e7f696fc",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-sk",
                rev: "5831d075e9477ac6a77eb4b038758b82746fb99d",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "1fe1beb8c0d7836d9ca4025fd4416b23d35c32cd85a8ab4897610f8dacf32ba8",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 789505,
                        sha256: "5c99b71996bd305cc2e602ff498b6232d97c2c8fc650c7797b8a6fcc71b2bc8e",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1461131,
                        sha256: "f933c1374027186cbde2ce24517e307b27751c3e3fe910262653656037ff2326",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 301851716,
                        sha256: "326e8c3f3561928de17f3ef4f93f920a97b9b5bfbc189c70d65072371287c021",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "hu",
        models: [
            Source {
                name: "opus-mt-hu-en",
                rev: "45169fa1016f77b226479d5022f51d7e778728c0",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "e86094e8d8ff7ffe31b01465c6f4b37ea342046858be7521bc0ceb390045d6ac",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 849914,
                        sha256: "e53f2bee88be1541f17929195532b2c77e5bbd6dfb69aeb0589d8fe457a2b6b0",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1565798,
                        sha256: "b781384d9efa8501f919909e6bf9d23138b874503ca9e795b30ec567767e4b92",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 306975560,
                        sha256: "65104eb75a197ffe96d802fc8203f09eff681040e69713df75b7532d32384552",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-hu",
                rev: "d25b056bbf18fbe7d956e295989c870342b56390",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "9ad007e04e04dfd157696ea883c895e59131a069859316d1d9cea1073c1a8d0c",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 792289,
                        sha256: "13cc9fb7021c19589da645e49efe801ac51e9563db22da262156903ddb98cea6",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1565798,
                        sha256: "b781384d9efa8501f919909e6bf9d23138b874503ca9e795b30ec567767e4b92",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 306975560,
                        sha256: "1bc13ce06cb34e227cecbc241ab3eac2204cb8a7faffae379ceb1e6dff888fb9",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "bg",
        models: [
            Source {
                name: "opus-mt-bg-en",
                rev: "a8ccc2cc1ccd24d61a918d478684b463f41d9db0",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "9040d36f3fa8f83b4bd6d41f6e266e2dedec5caae6465df2f2950b4311038a98",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 996555,
                        sha256: "9202eec091ecf5fd631e32495baf1bf94d90051e9bf65d37586ba0ef05b5d2ee",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 2349535,
                        sha256: "23c05161c798ab9dede197e7bd86103076d96bc13c423cff4cd780f252389b90",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 305520692,
                        sha256: "15aff33b03f5b282a5833188ea178445fbb509b2c58c618d25b0f6d818463a17",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-bg",
                rev: "e451832aacc2e4643b6b3ca1fd1d5b25ee65c548",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1394,
                        sha256: "2bf71f84c855103045138cc741b47605d5055fd8f7210ec772a2826764bc5989",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 791438,
                        sha256: "36b6f4ca28f4a3521b39c9a1d81902a4465bb850eeff2de263437b8dd6d40761",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 2327835,
                        sha256: "7c958688f71e783f2892876ee0db282dfb28d2df2ef976a16c97b73ca055b29e",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 305305232,
                        sha256: "ae5f35fe3cbef321abac63656e4ff04a0710c11acb9302535b463015221d9c04",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "ru",
        models: [
            Source {
                name: "opus-mt-ru-en",
                rev: "a64ede1bb36da5cd0c7bd9c25d0ee07ce5612124",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "5ea76c78596bce8fe005ef89e00de0924bf83e5f532ce08784ff1fcefb699f5f",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 1080169,
                        sha256: "745998e51ba5b058e38b7ac7765c25c43ed5c1c39cc92b27163b9b2e323c9d7c",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 2601758,
                        sha256: "33e95da3be3fa3b50169c4c46693ba2f29fbf4cb29d99044bd07d72d181fa1e9",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 306967352,
                        sha256: "f73dc54675dc0da9ca6704098cadae3562f149b1f93c585f9d21c8502a760421",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-ru",
                rev: "20855d62f29510b14806b4f3eddb289a0065d556",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "d3c2614fd099bc228fc257f047e46f1c6f94cfa12dee2b1fa6c0d780e6b7de06",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 802781,
                        sha256: "16bebef1389a0b8ab452772c4e35b9e605e5713f8ac7baa71ca701394eaa086d",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 2601758,
                        sha256: "33e95da3be3fa3b50169c4c46693ba2f29fbf4cb29d99044bd07d72d181fa1e9",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 306967352,
                        sha256: "eb1bda1bd256f14435a7904373c8bec152fe0e85503bcecf1373b2dd47a9f03f",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "uk",
        models: [
            Source {
                name: "opus-mt-uk-en",
                rev: "dd9f5251b1b6e73bb203c778d5583c38557daba2",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "767e0b7efd335e781b1856016407265b27893d213730e16b855a45ee85e17eda",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 1007605,
                        sha256: "7abd810b2df512e6fe177f8732f6a0b3107614c84cc52aba20d805b4cb0d7b6b",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 2367710,
                        sha256: "992107700c8d5f9c1f41fefa5f72ac99050852b39d75f979c4e59344e6d14770",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 305056940,
                        sha256: "a52221e3b5ad186534f7c5a1af9b89728db9ce437a08cb27c8c5d08cc13014fc",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-uk",
                rev: "d87904e0b1cc1e486da4356a01624c29a33bc1bf",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "01e85031802ed6c76c43269c1dc8b641d20687b79cc700cce8cd6e0440af8854",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 808645,
                        sha256: "4dc9157e60a15b157c2f3d5f892379b03aed111c9fe8ac7cecba5f2aa56379ef",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 2367710,
                        sha256: "992107700c8d5f9c1f41fefa5f72ac99050852b39d75f979c4e59344e6d14770",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 305056940,
                        sha256: "a0f3dce3522dc5c991df5b5b669c38622234f56eeb048f9ad95e4459e0ee1088",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "et",
        models: [
            Source {
                name: "opus-mt-et-en",
                rev: "663c5fc68a0871226964cfa6405cc9b11d37dd69",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "b68da3932bbcee6bf609d059b65719771837dd91f98389d434bc088e50f7217d",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 825401,
                        sha256: "b05417b4c18e87989d8663eda31cf4bd869da34229fb8d44c57d9e51000799be",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1346529,
                        sha256: "f599551ea87b3f80a9f7f2d9ac087eb6369425a96ec6ca2a3461d7190bd12b4f",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 299473448,
                        sha256: "1bad54085e680eeb8432f58991bb1fa7f081ec12cc36a7650e467c840bf8ab8f",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-et",
                rev: "85ac5f70f5452595698bde4f3690ceeb289eeade",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "d148fd69f8e8f379c44384a2c8373e5a0cd63c07cce4c08e9c1cdcda003b4095",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 790422,
                        sha256: "01d2ea871fe6a4d9f4363901dfb67c18af1fb3b1797b5cc40ddcee2dd96855de",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1346529,
                        sha256: "f599551ea87b3f80a9f7f2d9ac087eb6369425a96ec6ca2a3461d7190bd12b4f",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 299473448,
                        sha256: "47dff494ef2b31befb1867b7a4d7219417317e7a07bd8a35d6a1ef38b9eb50d7",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "ca",
        models: [
            Source {
                name: "opus-mt-ca-en",
                rev: "ebb26adc9ed06ea75b1487c2e68b35bbb143b991",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "8f9230e4f8b697895057cf59a8008b3ee305f7a4df308491d02605cf8d27c26d",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 814870,
                        sha256: "409f03510e6350f2988dc008104cd7a441111c4987e965c9263f708621bf6e48",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1255491,
                        sha256: "c1a122f91db8aeecf7c290ddb0d13abc673e61eb441f53ef287d3d424e0515df",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 292063676,
                        sha256: "9a3d9ed452dd30e9785ac88e88b139c14671894673a2003b6a01bf4c82eab9ff",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-ca",
                rev: "4f09ad09fd2b9b4ebfece4d402d685e614bb8a99",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "dcdb30444c6f64e23c241d05c560f2c54a10e0c78457b6041d71cbacced4d113",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 802656,
                        sha256: "a89fc5871f11a5e363f6e3a92880c146e9b0158918cb5b8acefb8dcb031229dc",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1255491,
                        sha256: "c1a122f91db8aeecf7c290ddb0d13abc673e61eb441f53ef287d3d424e0515df",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 292063676,
                        sha256: "d8ea5d5652bd554a61501852ade8704bb769b922d7c188efc3aa7648744a8aff",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "zh",
        models: [
            Source {
                name: "opus-mt-zh-en",
                rev: "fd5d2c871cd77baae0ec4beb338a3f3eee806af0",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1394,
                        sha256: "d84104da12fa90da59acf5f0ed30428e43cc92574ad4b9dfa88f29eeeb2b0eb0",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 804677,
                        sha256: "e27a3a1b539f4959ec72ea60e453f49156289f95d4e6000b29332efc45616203",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1617902,
                        sha256: "c0f79ee4c413ccb24cd808a0c727eec85b2130f23c82802cc1caeb07b5f0d458",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 312062468,
                        sha256: "7a567d463a3a859ff4e86ce44e590672b2425ed6cd93b4b98c382cd2b9bea625",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-zh",
                rev: "408d9bc410a388e1d9aef112a2daba955b945255",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1403,
                        sha256: "bcae8ed74fed77fb51c58462b62397fee6b1a1a34aece79183a0dd02ad329e71",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 806435,
                        sha256: "5775ddc9e3ff2fae91554da56468ad35ff56edaba870fea74447bc7234bfdaa8",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1617791,
                        sha256: "f1832ca38f7aa158b9a944ff583e4901e6b37d9d4ad9241f03838e636e9cfb03",
                    },
                    RemoteFile {
                        name: "flax_model.msgpack",
                        size: 309944974,
                        sha256: "de82e23a535e25896ebdf433ec2b72706a428fb8e47bb87ef72531426edee8c3",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "ar",
        models: [
            Source {
                name: "opus-mt-ar-en",
                rev: "0a22ca0fe7fd0e314f4913e606ddc07d1ff9d718",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "06cabbca37bf0400a704980ca801611e391fc7b0554ffa17dfac0cb6847724c8",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 917407,
                        sha256: "484f7210e5f4466f7e5b99b660717c6ef2a6f90746196daba507795b69dc02a2",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 2131576,
                        sha256: "86c1be5815ed18525f1cff0d0e08bcd73e95f25f9f924ea7552113d005560b7c",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 307615784,
                        sha256: "3066ee362fc2f4409478538e0655d6b8cbf446b41a826e0659a8713bb687824f",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-ar",
                rev: "be6759628ad2192438f2a32c814c35274045d02d",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1394,
                        sha256: "5ad2d279ce62ceccb34e99e9f2bb455160c5cafa9a438e2f5785f0e9b61f48f0",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 801074,
                        sha256: "81ac8f4d4ee271f4109eac67560a31f9c2d555463ba5eb95c32c6deee8ae819d",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 2117975,
                        sha256: "4d37dc73c39de711bd63330835a54ac9751b2702e737ee0943f01a2e66256115",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 307550120,
                        sha256: "bbaab02ca243b6bee164ed43566e0649faca538657422527e37ee087d2a5bc48",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "hi",
        models: [
            Source {
                name: "opus-mt-hi-en",
                rev: "fdd51234073cd464cfc5fc576d0d884cebf67239",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "52a945ef5f9c60709e1e3b96f0b511494bc0982e1104dba704ddce9dd7117a8d",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 1057690,
                        sha256: "dd20c408ec4568361d32b41080e3f6fa5b82895135bd25c2747b2c145e42d07b",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 2057453,
                        sha256: "bbc167fda76fc7c321331b91024aad4ea1132bed7fab4246ba9819b7cab5ad97",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 304113020,
                        sha256: "f09b34ffd2a8d8046b1733f0796b390c2be907effe7c6a70ed162fae9aae68c1",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-hi",
                rev: "96ffbe4e4d6f88d6667a6e2a9370298106cf3cf0",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1394,
                        sha256: "46dc173b18926410535ce00d2a236794f454020a1d1ea8d2d3ec2292d1450b14",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 812240,
                        sha256: "fd4e951487aed00bae6a6c2ee4ef5d8d1db05fd098b19b608046c9334b58d24d",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 2097836,
                        sha256: "4126cbe0acd618982d6fe0c4f7866b6545617b0dfe0ebdd17b8a2dcd4d9a3a40",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 305801816,
                        sha256: "46ae1116913bce01c9d848a78f62da2bd986d728bced4dc1acd5fedf4338ac5e",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "id",
        models: [
            Source {
                name: "opus-mt-id-en",
                rev: "947e42c797c1b1b9bb4fdaae981fa68d1adcf80b",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "67f3a2b3217f56c5efe717b2374a5599761de7a3103d8b9411a0521971056446",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 800687,
                        sha256: "2a8fefe71c7f26cb0c6aa1b9f0cc0f8d18006b20fe41c547af7f25b9c8333465",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1257390,
                        sha256: "efb902a7e49901c3dcdd4b2e079419a0b6ccead3fa2cabfc90820ce1ba66e157",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 291121808,
                        sha256: "e71532a9cfa6392e7ac5f725d3c9dc82ff6c5a9701b1a407db6dcc25bf4440ce",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-id",
                rev: "563d0e360f50f967ddd0c8f23666ec4558801e69",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1381,
                        sha256: "4d7c70eec8e533308818fdbe89db9c56a3163261aa0c07ccc8d1dca4c1e5bd9d",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 795925,
                        sha256: "e88300911c2c573ec5526777a1e84bae698d20925b82dcef9c7248bb0e537ed0",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1257390,
                        sha256: "efb902a7e49901c3dcdd4b2e079419a0b6ccead3fa2cabfc90820ce1ba66e157",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 291121808,
                        sha256: "e7169d62592b1a9fefe33b7ab410c5d1d619e39b1df3d07175bd57d8331ffcd1",
                    },
                ],
            },
        ],
    },
    Pack {
        code: "vi",
        models: [
            Source {
                name: "opus-mt-vi-en",
                rev: "0ac68ebfb1aeb7672c18bf2c8610d0f332d7ac2f",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1394,
                        sha256: "190952e8f4572e178fcadb653ed67d6fbd5a576e411b1e69b5628f3a287d40e4",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 756168,
                        sha256: "9deb4f9e7bebd3876f4bf238a9eb52f2f95a1fba2210a4a8239167c55c15e67d",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1193597,
                        sha256: "07a5fc1c8250a0e3c503933c7e9145a9b5eee83afde59408deb640d85d79dd35",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 288952844,
                        sha256: "a76be921841367e1a5a0db52e573a119fd5c582696d3a12ad7cc35e5437c0617",
                    },
                ],
            },
            Source {
                name: "opus-mt-en-vi",
                rev: "269d795701557cdd499cb99783085cf9c9a3cd50",
                files: &[
                    RemoteFile {
                        name: "config.json",
                        size: 1394,
                        sha256: "35c71b5df9d9500d557b90c7f6b86d5277ad8fdbff63a26a621ca1d23f4045fb",
                    },
                    RemoteFile {
                        name: "source.spm",
                        size: 808612,
                        sha256: "fdd9ab486ca8ea060967105e30db7d8928729c78a67448711d2dd7c3153b37cd",
                    },
                    RemoteFile {
                        name: "vocab.json",
                        size: 1192392,
                        sha256: "fccff4e1ed80f65ebc9b13352b4267d735595480c86378534d46f9d4b3c13c7c",
                    },
                    RemoteFile {
                        name: "model.safetensors",
                        size: 288842036,
                        sha256: "652c300995d3d074c558ed799dbc9afba223441102cafee1b20e1c52a825d3a6",
                    },
                ],
            },
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue() {
        assert!(PACKS.len() >= 20);
        for p in PACKS {
            assert_eq!(p.models[0].name, format!("opus-mt-{}-en", p.code));
            assert_eq!(p.models[1].name, format!("opus-mt-en-{}", p.code));
            for m in &p.models {
                assert_eq!(m.rev.len(), 40);
                assert_eq!(m.files.iter().filter(|f| is_weights(f.name)).count(), 1);
                assert!(m.files.iter().all(|f| f.sha256.len() == 64 && f.size > 0));
            }
        }
        assert_eq!(target_token("opus-mt-en-zh", "zh-TW"), Some(">>cmn_Hant<<"));
        assert_eq!(target_token("opus-mt-en-zh", "zh"), Some(">>cmn_Hans<<"));
        assert_eq!(target_token("opus-mt-en-de", "de"), None);
    }
}
