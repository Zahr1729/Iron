use std::{path::PathBuf, sync::Arc};

use eframe::egui::TextBuffer;
use ron::from_str;
use symphonia::core::formats::Track;

use crate::audio::{dag::EffectDAG, effects::Effect};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct SceneTrack {
    file_name: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Scene {
    tracks: Vec<SceneTrack>,
}

impl Scene {
    pub fn generate_effect_dag(&self) -> EffectDAG {
        todo!()
    }

    pub fn from_effect_dag() -> Self {
        todo!()
    }

    pub fn from_track(path: PathBuf) -> Self {
        let s = format!("Scene(tracks: [SceneTrack (file_name: {:?} )])", path);
        let scene: Scene = ron::from_str(s.as_str()).unwrap();

        scene
    }
}

#[cfg(test)]
mod test {
    use std::{fs::File, io::Read, path::PathBuf, str::FromStr};

    use crate::scene::Scene;

    #[test]
    fn test_use_file_name() {
        let path = r"..\mp3s\C_major";
        let scene = Scene::from_track(PathBuf::from(path));

        assert!(
            path == scene.tracks[0].file_name.to_str().unwrap(),
            "File name data not recovered"
        );
    }

    #[test]
    fn test_create_file() {
        let scene_path = r"example_file.ron";
        let file_path = r"..\mp3s\C_major";

        let s = Scene::from_track(PathBuf::from(file_path));

        let f = File::options()
            .create(true)
            .write(true)
            .open(scene_path)
            .expect("Failed opening file");

        ron::Options::default()
            .to_io_writer_pretty(f, &s, ron::ser::PrettyConfig::new())
            .expect("Failed to write to file");
    }

    #[test]
    fn test_read_file_name() {
        let scene_path = r"./example_file.ron";

        let strng = r#"(
                tracks: [
                    (
                        file_name: "..\\mp3s\\C_major",
                    ),
                ],
            )"#;

        let f = File::open(scene_path).expect("could not open file");

        let s1 = ron::Options::default()
            .from_reader::<File, Scene>(f)
            .unwrap();

        let s2: Scene = ron::from_str(&strng).unwrap();
        assert!(s1 == s2, "read data does not agree with what should exist");

        // match f.read_to_string(&mut strng) {
        //     Ok(_) => {
        //         println!("{strng}");
        //     }
        //     Err(_) => assert!(false, "data not extracted"),
        // };
    }
}
