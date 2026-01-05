//! Standard Dictionary library

use std::collections::HashMap;

/// Create an empty dictionary
pub fn dict_new<K, V>() -> HashMap<K, V>
where
    K: std::hash::Hash + std::cmp::Eq,
{
    HashMap::new()
}

/// Get dictionary length
pub fn dict_len<K, V>(dict: &HashMap<K, V>) -> usize {
    dict.len()
}

/// Check if dictionary is empty
pub fn dict_is_empty<K, V>(dict: &HashMap<K, V>) -> bool {
    dict.is_empty()
}

/// Get value for key
pub fn dict_get<'a, K, V>(
    dict: &'a HashMap<K, V>,
    key: &K,
) -> Option<&'a V>
where
    K: std::hash::Hash + std::cmp::Eq,
{
    dict.get(key)
}

/// Set value for key
pub fn dict_insert<K, V>(
    dict: &mut HashMap<K, V>,
    key: K,
    value: V,
) where
    K: std::hash::Hash + std::cmp::Eq,
{
    dict.insert(key, value);
}

/// Remove key from dictionary
pub fn dict_remove<K, V>(
    dict: &mut HashMap<K, V>,
    key: &K,
) -> Option<V>
where
    K: std::hash::Hash + std::cmp::Eq,
{
    dict.remove(key)
}

/// Check if key exists
pub fn dict_contains<K, V>(
    dict: &HashMap<K, V>,
    key: &K,
) -> bool
where
    K: std::hash::Hash + std::cmp::Eq,
{
    dict.contains_key(key)
}

/// Clear dictionary
pub fn dict_clear<K, V>(dict: &mut HashMap<K, V>) {
    dict.clear()
}

/// Get keys
pub fn dict_keys<K, V>(dict: &HashMap<K, V>) -> Vec<K>
where
    K: Clone + std::hash::Hash + std::cmp::Eq,
{
    dict.keys().cloned().collect()
}

/// Get values
pub fn dict_values<K, V: Clone>(dict: &HashMap<K, V>) -> Vec<V> {
    dict.values().cloned().collect()
}
