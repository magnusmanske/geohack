/// Macro to insert multiple key-value pairs into a HashMap
///
/// Usage:
/// ```
/// insert_map!(map, {
///     "key1" => value1,
///     "key2" => value2.to_string(),
/// });
/// ```
#[macro_export]
macro_rules! insert_map {
    ($map:expr, { $($key:expr => $value:expr),* $(,)? }) => {
        $(
            $map.insert($key.to_string(), $value.to_string());
        )*
    };
}

/// Macro to create a HashMap with the given key-value pairs
///
/// Usage:
/// ```
/// let map = hash_map! {
///     "key1" => "value1",
///     "key2" => "value2",
/// };
/// ```
#[macro_export]
macro_rules! hash_map {
    ({ $($key:expr => $value:expr),* $(,)? }) => {
        {
            let mut map = std::collections::HashMap::new();
            $(
                map.insert($key.to_string(), $value.to_string());
            )*
            map
        }
    };
}
