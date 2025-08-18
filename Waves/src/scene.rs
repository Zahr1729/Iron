use std::{path::PathBuf, sync::Arc};

use crate::audio::effects::{Effect, Gain};
use crate::audio::{dag::EffectDAG, effects::Zero};
use crate::common::track::Track;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum NodeType {
    // Output is what everything feeds into it WILL be at index zero.
    Zero,
    Track {
        file_path: PathBuf,
        // start : usize,
    },
    Gain {
        #[allow(non_snake_case)]
        dB: f32,
        input: usize, // The index of our element
    },
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Scene {
    start_index: Option<usize>,
    nodes: Vec<NodeType>,
}

impl Scene {
    fn expand_dag(&self, index: usize, dag: &mut EffectDAG) -> Arc<dyn Effect> {
        match &self.nodes[index] {
            NodeType::Zero => dag.add_effect(Zero),
            NodeType::Gain { dB, input } => {
                let input = self.expand_dag(*input, dag);
                dag.add_effect(Gain::new(crate::common::dB(*dB), input))
            }
            NodeType::Track { file_path } => {
                dag.add_effect(Track::get_data_from_mp3_path(file_path.clone(), None).unwrap())
            }
        }
    }

    pub fn generate_effect_dag(&self) -> EffectDAG {
        match self.start_index {
            None => {
                println!("there is no start index");
                EffectDAG::new(0, vec![Arc::new(Zero)])
            }
            Some(i) => {
                let mut dag = EffectDAG::new(i, vec![]);
                self.expand_dag(i, &mut dag);

                // force the index to be the last thing placed on the dag (which must be the root)
                dag.set_root_index(dag.nodes().len() - 1);

                dag
            }
        }
    }

    pub fn from_effect_dag() -> Self {
        todo!()
    }

    pub fn from_track(path: PathBuf) -> Self {
        Self {
            start_index: Some(0),
            nodes: vec![NodeType::Track { file_path: path }],
        }
    }
}

#[cfg(test)]
mod test {
    use std::{any::Any, fs::File, path::PathBuf};

    use super::*;

    #[test]
    fn test_use_file_name() {
        let path = r"..\mp3s\C_major";
        let scene = Scene::from_track(PathBuf::from(path));

        match scene.nodes[0].clone() {
            NodeType::Track { file_path: p } => {
                //println!("{:?}, {path}", p);
                assert!(p.to_str().unwrap() == path, "Path not saved appropriately")
            }
            _ => assert!(false, "Entry at index 1 is not a track"),
        }
    }

    #[test]
    fn test_create_file() {
        let scene_path = r"scene\example_file.ron";
        let file_path = r"mp3s\C_major.mp3";

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
        let scene_path = r"scene\example_file.ron";

        let string = r#"(
    start_index: Some(0),
    nodes: [
        Track(
            file_path: "mp3s\\C_major.mp3",
        ),
    ],
)"#;

        let f = File::open(scene_path).expect("could not open file");

        let s1 = ron::Options::default()
            .from_reader::<File, Scene>(f)
            .unwrap();

        let s2: Scene = ron::from_str(&string).unwrap();
        assert!(s1 == s2, "read data does not agree with what should exist");

        // match f.read_to_string(&mut strng) {
        //     Ok(_) => {
        //         println!("{strng}");
        //     }
        //     Err(_) => assert!(false, "data not extracted"),
        // };
    }

    #[test]
    fn test_generate_dag() {
        let db = 2.0;
        let file_path = PathBuf::from(r"mp3s\C_major.mp3");

        let scene = Scene {
            start_index: Some(1),
            nodes: vec![
                NodeType::Track {
                    file_path: file_path.clone(),
                },
                NodeType::Gain { dB: db, input: 0 },
            ],
        };

        // let scene = Scene {
        //     start_index: Some(0),
        //     nodes: vec![NodeType::Zero],
        // };

        let dag = scene.generate_effect_dag();

        // we being a bit silly
        assert_eq!(dag.root_index(), 1);
        let node_zero = &*dag.nodes()[0] as &dyn Any;
        assert_eq!(
            node_zero
                .downcast_ref::<Track>()
                .unwrap()
                ._file_path()
                .unwrap(),
            file_path
        );
        // let node_one = &*dag.nodes()[1] as &dyn Any;
        // assert_eq!(node_one.downcast_ref::<Gain>().unwrap().gain().0, db);
    }
}
