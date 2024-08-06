use std::str::FromStr;

use crate::{
    config::Widget,
    plug::{
        common::shell_cmd,
        system::{init_mem_info, init_system_info, register_disk_partition},
    },
};

use super::common::{color_translate, dt_frame_rate};
use educe::Educe;
use gtk::gdk::RGBA;
use serde::{Deserialize, Deserializer};
use serde_jsonrc::Value;

pub const NAME: &str = "ring";

#[derive(Debug)]
pub enum RingPreset {
    Ram,
    Swap,
    Cpu,
    Battery,
    Disk(String),
    Custom(RingCustom),
}

pub type UpdateTask = Box<dyn Send + FnMut() -> Result<(f64, Option<String>), String>>;

#[derive(Deserialize, Educe)]
#[educe(Debug)]
pub struct RingCustom {
    #[educe(Debug(ignore))]
    #[serde(default)]
    #[serde(deserialize_with = "update_task_interval")]
    pub update_with_interval_ms: Option<(u64, UpdateTask)>,
}

#[derive(Deserialize, Educe)]
#[educe(Debug)]
pub struct RingCommon {
    #[serde(default = "dt_r")]
    pub radius: f64,
    #[serde(default = "dt_rw")]
    pub ring_width: f64,
    #[serde(default = "dt_bg")]
    #[serde(deserialize_with = "color_translate")]
    pub bg_color: RGBA,
    #[serde(default = "dt_fg")]
    #[serde(deserialize_with = "color_translate")]
    pub fg_color: RGBA,

    #[serde(default = "dt_frame_rate")]
    pub frame_rate: u32,
    #[serde(default = "dt_tt")]
    pub text_transition_ms: u64,

    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub font_family: Option<String>,
}

fn dt_r() -> f64 {
    13.0
}
fn dt_rw() -> f64 {
    5.0
}
fn dt_bg() -> RGBA {
    RGBA::from_str("#9F9F9F").unwrap()
}
fn dt_fg() -> RGBA {
    RGBA::from_str("#F1FA8C").unwrap()
}
fn dt_tt() -> u64 {
    100
}

#[derive(Debug)]
pub struct RingConfig {
    pub common: RingCommon,
    pub preset: RingPreset,
}
unsafe impl Send for RingConfig {}
unsafe impl Sync for RingConfig {}
impl Drop for RingConfig {
    fn drop(&mut self) {
        log::info!("Dropping RingConfig");
    }
}

pub fn visit_config(d: Value) -> Result<Widget, String> {
    let preset = {
        let preset = d.get("preset").ok_or("Ring preset not provided")?;
        let preset_type = {
            if let Some(s) = preset.as_str() {
                s
            } else {
                preset
                    .as_object()
                    .ok_or("Preset must be a string or object")?
                    .get("type")
                    .ok_or("Preset type not provided")?
                    .as_str()
                    .ok_or("Preset type must be a string")?
            }
        };
        match preset_type {
            "ram" => {
                init_mem_info();
                RingPreset::Ram
            }
            "swap" => {
                init_mem_info();
                RingPreset::Swap
            }
            "cpu" => {
                init_system_info();
                RingPreset::Cpu
            }
            "battery" => {
                init_system_info();
                RingPreset::Battery
            }
            "disk" => {
                let partition = preset
                    .get("partition")
                    .unwrap_or(&Value::String("/".to_string()))
                    .as_str()
                    .ok_or("partition must be string")?
                    .to_string();
                init_system_info();
                register_disk_partition(partition.clone());
                RingPreset::Disk(partition)
            }
            "custom" => {
                RingPreset::Custom(super::common::from_value::<RingCustom>(preset.clone())?)
            }
            _ => {
                return Err(format!("Unknown preset type: {preset_type}"));
            }
        }
    };
    let common = super::common::from_value::<RingCommon>(d)?;
    Ok(Widget::Ring(Box::new(RingConfig { common, preset })))
}

fn update_task_interval<'de, D>(d: D) -> Result<Option<(u64, UpdateTask)>, D::Error>
where
    D: Deserializer<'de>,
{
    struct EventMapVisitor;
    impl<'de> serde::de::Visitor<'de> for EventMapVisitor {
        type Value = Option<(u64, UpdateTask)>;

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let ms = seq.next_element()?.unwrap();
            let ut = seq.next_element()?.unwrap();
            Ok(Some((ms, create_update_task(ut))))
        }

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("vec of tuples: (key: number, command: string)")
        }
    }
    d.deserialize_any(EventMapVisitor)
}
fn create_update_task(value: String) -> UpdateTask {
    Box::new(move || {
        let value = value.clone();
        let a = shell_cmd(value)?;
        Ok(if let Some((p, text)) = a.split_once('\n') {
            (
                f64::from_str(p).map_err(|e| format!("Fail to convert result({a}) to f64: {e}"))?,
                Some(text.to_string()),
            )
        } else {
            (
                f64::from_str(&a)
                    .map_err(|e| format!("Fail to convert result({a}) to f64: {e}"))?,
                None,
            )
        })
    })
}
