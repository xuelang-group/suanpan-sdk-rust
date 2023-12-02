use std::collections::HashMap;

const ARG_NAME_PREFIX: &str = "--";
const ARG_VALUE_PREFIX: char = '\'';

pub fn get_args(input: &str) -> HashMap<String, String> {
    let mut args_map = HashMap::new();
    let param_array: Vec<&str> = input.split_whitespace().collect();
    add_param_array(&mut args_map, &param_array);
    args_map
}

fn add_param_array(args_map: &mut HashMap<String, String>, param_array: &[&str]) {
    let mut i = 0;
    while i < param_array.len() {
        if param_array[i].starts_with(ARG_NAME_PREFIX) {
            let key = param_array[i].to_string();
            if i + 1 < param_array.len() {
                let value = param_array[i + 1].trim_matches(ARG_VALUE_PREFIX);
                args_map.insert(key, value.to_string());
            } else {
                args_map.insert(key, "".to_string());
            }
            i += 2; // Skip the value we just read.
        } else {
            i += 1; // Skip this parameter as it's not a key.
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_args() {
        let args =
            get_args("--stream-user-id 'shanglu' --stream-app-id '64049' --dw-hive-password ''");
        println!("{:?}", args);
        assert_eq!(args.get("--dw-hive-password").unwrap(), "");
        assert_eq!(args.get("--stream-user-id").unwrap(), "shanglu");
        assert_eq!(args.get("--stream-app-id").unwrap(), "64049");
    }
}
