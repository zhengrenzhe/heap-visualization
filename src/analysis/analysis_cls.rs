use wasm_bindgen::prelude::*;

use crate::analysis::filter_define::{FilterCondition, SameStringCondition};
use crate::analysis::result_define::{EdgeDetailInfo, NodeDetailInfo};
use crate::decode_snapshot::decode::decode;
use crate::decode_snapshot::snapshot_define::Node;
use crate::decode_snapshot::snapshot_provider::SnapshotProvider;
use crate::utils::consts::STRING_NODE_TYPE;
use crate::utils::decode::decode_js_value;
use crate::utils::log::Log;
use crate::utils::search::count_same_string;

#[wasm_bindgen]
pub struct SnapshotAnalysis {
    buffer: Vec<u8>,
    provider: SnapshotProvider,
}

#[wasm_bindgen]
impl SnapshotAnalysis {
    #[wasm_bindgen(constructor)]
    pub fn new(byte_length: usize) -> Self {
        SnapshotAnalysis {
            buffer: vec![0; byte_length as usize],
            provider: SnapshotProvider {
                nodes: vec![],
                edges: vec![],
                strings: vec![],
                edge_count: 0,
                node_count: 0,
                edge_types: vec![],
                node_types: vec![],
            },
        }
    }

    #[wasm_bindgen]
    pub fn set_buffer(&mut self, index: usize, value: u8) {
        self.buffer[index] = value;
    }

    #[wasm_bindgen]
    pub fn start_parse(&mut self) {
        Log::info("parsing");

        match decode(&self.buffer) {
            Some(provider) => {
                self.provider = provider;
                let nodes_count = &self.provider.nodes.len();
                let edges_count = &self.provider.edges.len();
                Log::info2_usize("parsing-done", *nodes_count, *edges_count);
            }
            None => {
                panic!("decode error");
            }
        }

        self.buffer.clear();
    }

    #[wasm_bindgen]
    pub fn get_graph(&self, cond: &JsValue) -> JsValue {
        let cond: FilterCondition = decode_js_value(cond);

        Log::info("searching");

        let nodes = SnapshotAnalysis::filter_nodes(&self.provider, cond);

        let (result_node, result_edge) =
            SnapshotAnalysis::get_children_graph(&self.provider, &nodes);

        Log::info2_usize("got-nodes", result_node.len(), result_node.len());

        SnapshotAnalysis::convert_graph_to_js(&result_node, &result_edge)
    }

    #[wasm_bindgen]
    pub fn get_node_detail(&self, id: u32) -> JsValue {
        let (strings, strings_len) = self.provider.get_strings();
        let (node_types, node_types_len) = self.provider.get_node_types();

        match self.provider.nodes.iter().find(|node| node.id == id) {
            Some(node) => JsValue::from_serde(&NodeDetailInfo::from_node(
                node,
                strings,
                strings_len,
                node_types,
                node_types_len,
            ))
            .expect("failed convert NodeDetailInfo"),
            None => JsValue::null(),
        }
    }

    #[wasm_bindgen]
    pub fn get_edge_detail(&self, edge_index: usize) -> JsValue {
        let (strings, strings_len) = self.provider.get_strings();
        let (edge_types, edge_types_len) = self.provider.get_edge_types();

        match self
            .provider
            .edges
            .iter()
            .find(|edge| edge.edge_index == edge_index)
        {
            None => JsValue::null(),
            Some(edge) => JsValue::from_serde(&EdgeDetailInfo::from_edge(
                edge,
                strings,
                strings_len,
                edge_types,
                edge_types_len,
            ))
            .expect("failed convert EdgeDetailInfo"),
        }
    }

    #[wasm_bindgen]
    pub fn get_same_string_value_nodes(&self, cond: &JsValue) -> JsValue {
        let cond: SameStringCondition = decode_js_value(cond);

        Log::info("searching");

        let (node_types, node_types_len) = self.provider.get_node_types();
        let (strings, strings_len) = self.provider.get_strings();

        let use_excludes = !cond.excludes.is_empty();
        let use_includes = !cond.includes.is_empty();

        // node name, node index
        let string_nodes: Vec<(&str, usize)> = self
            .provider
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(node_index, node)| {
                let node_type = node.get_node_type(node_types, node_types_len);
                if STRING_NODE_TYPE.contains(&node_type) {
                    let node_name = node.get_node_name(strings, strings_len);
                    if node_name.chars().count() < cond.minimum_string_len {
                        return None;
                    }
                    if use_excludes && cond.excludes.iter().any(|ex| node_name.contains(ex)) {
                        return None;
                    }
                    if use_includes && !cond.includes.iter().any(|inc| node_name.contains(inc)) {
                        return None;
                    }
                    Some((node_name, node_index))
                } else {
                    None
                }
            })
            .collect();

        let nodes: Vec<&Node> = count_same_string(&string_nodes, cond.more_than_same_times)
            .iter()
            .map(|node_index| &self.provider.nodes[*node_index])
            .collect();

        let (result_node, result_edge) =
            SnapshotAnalysis::get_children_graph(&self.provider, &nodes);

        Log::info2_usize("got-nodes", result_node.len(), result_edge.len());

        SnapshotAnalysis::convert_graph_to_js(&result_node, &result_edge)
    }
}
