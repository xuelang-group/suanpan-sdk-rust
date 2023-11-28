use serde::{Deserialize, Deserializer, Serialize, Serializer};
macro_rules! back_to_enum {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
    }) => {
        $(#[$meta])*
        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        #[repr(u32)]
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl std::convert::TryFrom<i32> for $name {
            type Error = ();

            fn try_from(v: i32) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as i32 => Ok($name::$vname),)*
                    _ => Err(()),
                }
            }
        }
    }
}

back_to_enum! { pub enum CodeType {
    SparkJava = 1, // 给 datas 的 java 组件用的
    SparkJavaNew = 7,
    SparkPython = 2,
    OdpsPython = 3,
    OdpsMr = 4,
    Python = 5,
    Java = 6,
    Notebook = 11,
    Runtime = 12,
    SupRuntime = 13,
    Datafile = 21,
    Sendfile = 31,
    Dashboard = 22,
    DashboardPage = 23,
    RpaCommon = 501,
    RpaExcel = 502,
    RpaBrowser = 503,
    RpaFlowControl = 504,
    RpaSystem = 505,
    RpaFile = 506,
    CompositeApp = 998,
    Composite = 999,
    Upstream = 1001,
    Downstream = 1002,
    Model = 10001,
    WebInput = 10002,
    WebOutput = 10003,
    Hardware = 50,
    HardwareNetwork = 51,
}
}

impl Serialize for CodeType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(*self as u32)
    }
}

impl<'de> Deserialize<'de> for CodeType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match DefTypeRepr::deserialize(deserializer)? {
            DefTypeRepr::Int(int) => {
                let r = CodeType::try_from(int as i32);
                match r {
                    Ok(v) => Ok(v),
                    _ => Err(serde::de::Error::custom(format!(
                        "Invalid value for CodeType, type raw value is {}",
                        int,
                    ))),
                }
            }
            DefTypeRepr::Str(_) => Ok(CodeType::Runtime), //heng: will code be "12"? typescripe only use if def.type == codetype.xxx(int repr) else will be send directly(maybe like runtime), that is urgly!!!!
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum DefTypeRepr {
    Int(u32),
    Str(String),
}
