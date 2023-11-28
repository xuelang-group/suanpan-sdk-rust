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
        let args = get_args("--stream-user-id 'shanglu' --stream-app-id '64049' --stream-node-id 'a002ad00c53211edb01585377faf4695' --stream-host 'spnextapi.xuelangyun.com' --stream-send-queue 'mq-master-shanglu-64049' --stream-recv-queue 'mq-master-shanglu-64049-a002ad00c53211edb01585377faf4695' --stream-send-queue-max-length '2000' --stream-send-queue-trim-immediately 'false' --mq-type 'redis' --mq-redis-host 'app-64049-redis' --mq-redis-port '6379' --mstorage-type 'redis' --mstorage-redis-default-expire '30' --mstorage-redis-host 'app-64049-redis' --mstorage-redis-port '6379'  --__mode 'online-edit'  --__change_pip_source 'https://pypi.mirrors.ustc.edu.cn/simple'  --storage-type 'oss' --storage-oss-endpoint 'https://oss-cn-beijing-internal.aliyuncs.com' --storage-oss-bucket-name 'suanpan' --storage-oss-access-id 'xxxx' --storage-oss-access-key '********' --storage-oss-temp-store '/suanpan' --storage-oss-global-store '/global' --dw-type 'hive' --dw-hive-host 'emr-header-1.cluster-79652' --dw-hive-port '10000' --dw-hive-database 'default' --dw-hive-auth '' --dw-hive-username '' --dw-hive-password '' ");
        println!("{:?}", args);
        assert_eq!(args.get("--dw-hive-password").unwrap(), "");
        assert_eq!(
            args.get("--stream-node-id").unwrap(),
            "a002ad00c53211edb01585377faf4695"
        );
        assert_eq!(args.get("--dw-hive-auth").unwrap(), "");
        assert_eq!(args.get("--storage-oss-access-key").unwrap(), "********");
    }
}
