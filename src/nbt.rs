//! NBT (Named Binary Tag) structures for Minecraft chunk data.
//!
//! These structures are serialized using fastnbt to create valid
//! Minecraft chunk data compatible with version 1.21.11.

use serde::{Deserialize, Serialize};

/// Minecraft data version for 1.21.11 (default).
/// Can be overridden by MC_DATA_VERSION env var.
pub fn get_data_version() -> i32 {
    std::env::var("MC_DATA_VERSION")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4671)
}

/// Main chunk structure - the root of NBT hierarchy in .mca files.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChunkData {
    #[serde(rename = "DataVersion")]
    pub data_version: i32,

    // Chunk coordinates (absolute, not relative to region)
    #[serde(rename = "xPos")]
    pub x_pos: i32,
    #[serde(rename = "zPos")]
    pub z_pos: i32,
    #[serde(rename = "yPos")]
    pub y_pos: i32,

    // "minecraft:full" tells the server the chunk is fully generated.
    #[serde(rename = "Status")]
    pub status: String,

    // Required timing fields
    #[serde(rename = "LastUpdate")]
    pub last_update: i64,

    #[serde(rename = "InhabitedTime")]
    pub inhabited_time: i64,

    // Light calculation status
    #[serde(rename = "isLightOn", default)]
    pub is_light_on: Option<i8>,

    // Vertical slices of the chunk (16 blocks high each)
    pub sections: Vec<Section>,
}

// --- Section (16x16x16 Cube) ---
#[derive(Debug, Serialize, Deserialize)]
pub struct Section {
    // Vertical index of this section (e.g., -4 for the bottom, up to 19)
    #[serde(rename = "Y")]
    pub y: i8,

    // The blocks inside this section
    // Optional because empty sections might omit this.
    // Also aliased to handle potential capitalization differences.
    #[serde(rename = "block_states", alias = "BlockStates", default)]
    pub block_states: Option<BlockStates>,

    // The biomes inside this section
    #[serde(rename = "biomes", alias = "Biomes", default)]
    pub biomes: Option<Biomes>,
}

// --- Smart Serialization Helper ---
mod opt_long_array {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use fastnbt::LongArray;

    pub fn serialize<S>(data: &Option<Vec<i64>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match data {
            Some(vec) => {
                if serializer.is_human_readable() {
                    // JSON: Serialize as plain list
                    vec.serialize(serializer)
                } else {
                    // NBT: Serialize as LongArray tag
                    LongArray::new(vec.clone()).serialize(serializer)
                }
            },
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<i64>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, Visitor, SeqAccess};
        use std::fmt;

        struct LongArrayOrListVisitor;

        impl<'de> Visitor<'de> for LongArrayOrListVisitor {
            type Value = Option<Vec<i64>>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an NBT LongArray or a list of longs")
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                let la = LongArray::deserialize(deserializer)?;
                Ok(Some(la.into_inner()))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element()? {
                    values.push(value);
                }
                Ok(Some(values))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(None)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_any(self)
            }
        }

        if deserializer.is_human_readable() {
            Option::<Vec<i64>>::deserialize(deserializer)
        } else {
            deserializer.deserialize_any(LongArrayOrListVisitor)
        }
    }
}

// --- Block Palette ---
// Minecraft uses "Paletted Storage". Instead of storing 4096 block IDs,
// it stores a list of unique blocks (Palette).
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockStates {
    pub palette: Vec<BlockState>,
    // Indices into the palette. Required if palette length > 1.
    #[serde(default, with = "opt_long_array")]
    pub data: Option<Vec<i64>>,
}

// --- Biome Palette ---
#[derive(Debug, Serialize, Deserialize)]
pub struct Biomes {
    pub palette: Vec<String>,
    // Indices into the palette. Required if palette length > 1.
    #[serde(default, with = "opt_long_array")]
    pub data: Option<Vec<i64>>,
}

// --- Single Block ---
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockState {
    #[serde(rename = "Name")]
    pub name: String,
    // Properties (like waterlogged, facing) are optional/omitted for MVP.
}