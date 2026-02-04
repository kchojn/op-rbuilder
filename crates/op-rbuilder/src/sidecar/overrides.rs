//! Helpers for building state overrides for sidecar simulations.

use reth_revm::State;
use revm::Database;
use serde_json::{Map, Value};
use tracing::warn;

/// Build JSON-RPC state overrides for the current in-progress builder state.
pub fn build_state_overrides<DB: Database>(state: &State<DB>) -> Value {
    let Some(transition_state) = state.transition_state.as_ref() else {
        if has_cache_modifications(state) {
            warn!(
                target: "sidecar",
                "state overrides: transition_state missing while cache has modifications"
            );
        }
        return Value::Object(Map::new());
    };

    if transition_state.transitions.is_empty() {
        if has_cache_modifications(state) {
            warn!(
                target: "sidecar",
                "state overrides: empty transition_state while cache has modifications"
            );
        }
        return Value::Object(Map::new());
    }

    let mut overrides = Map::new();

    for (addr, account) in transition_state.transitions.iter() {
        let mut state_diff = Map::new();
        for (slot, value) in account.storage.iter() {
            if value.is_changed() {
                state_diff.insert(
                    format!("{:#x}", slot),
                    Value::String(format!("{:#x}", value.present_value())),
                );
            }
        }

        let has_account_change = !account.status.is_not_modified() || account.has_new_contract().is_some();
        let has_storage_change = !state_diff.is_empty();
        if !has_account_change && !has_storage_change {
            continue;
        }

        let mut account_map = Map::new();
        if account.status.was_destroyed() || account.info.is_none() {
            account_map.insert("balance".to_string(), Value::String("0x0".to_string()));
            account_map.insert("nonce".to_string(), Value::String("0x0".to_string()));
            account_map.insert("code".to_string(), Value::String("0x".to_string()));
            account_map.insert("state".to_string(), Value::Object(Map::new()));
        } else if let Some(info) = account.info.as_ref() {
            account_map.insert(
                "balance".to_string(),
                Value::String(format!("0x{:x}", info.balance)),
            );
            account_map.insert(
                "nonce".to_string(),
                Value::String(format!("0x{:x}", info.nonce)),
            );
            if let Some((_hash, code)) = account.has_new_contract() {
                account_map.insert(
                    "code".to_string(),
                    Value::String(format!("0x{}", hex::encode(code.bytes_slice()))),
                );
            }
        }

        if !state_diff.is_empty() && !account_map.contains_key("state") {
            account_map.insert("stateDiff".to_string(), Value::Object(state_diff));
        }

        if !account_map.is_empty() {
            overrides.insert(addr.to_string(), Value::Object(account_map));
        }
    }

    Value::Object(overrides)
}

fn has_cache_modifications<DB: Database>(state: &State<DB>) -> bool {
    state
        .cache
        .accounts
        .values()
        .any(|account| !account.status.is_not_modified())
}
