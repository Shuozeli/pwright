//! Named key definitions for keyboard dispatch.
//! Mirrors PinchTab's namedKeyDefs in cdp.go.

use std::borrow::Cow;

pub struct KeyDef {
    pub code: Cow<'static, str>,
    pub virtual_key: i64,
    pub insert_text: &'static str,
}

pub fn get_key_def(name: &str) -> Option<KeyDef> {
    match name {
        "Enter" | "Return" => Some(KeyDef {
            code: Cow::Borrowed("Enter"),
            virtual_key: 13,
            insert_text: "\r",
        }),
        "Tab" => Some(KeyDef {
            code: Cow::Borrowed("Tab"),
            virtual_key: 9,
            insert_text: "\t",
        }),
        "Escape" => Some(KeyDef {
            code: Cow::Borrowed("Escape"),
            virtual_key: 27,
            insert_text: "",
        }),
        "Backspace" => Some(KeyDef {
            code: Cow::Borrowed("Backspace"),
            virtual_key: 8,
            insert_text: "",
        }),
        "Delete" => Some(KeyDef {
            code: Cow::Borrowed("Delete"),
            virtual_key: 46,
            insert_text: "",
        }),
        "ArrowLeft" => Some(KeyDef {
            code: Cow::Borrowed("ArrowLeft"),
            virtual_key: 37,
            insert_text: "",
        }),
        "ArrowRight" => Some(KeyDef {
            code: Cow::Borrowed("ArrowRight"),
            virtual_key: 39,
            insert_text: "",
        }),
        "ArrowUp" => Some(KeyDef {
            code: Cow::Borrowed("ArrowUp"),
            virtual_key: 38,
            insert_text: "",
        }),
        "ArrowDown" => Some(KeyDef {
            code: Cow::Borrowed("ArrowDown"),
            virtual_key: 40,
            insert_text: "",
        }),
        "Home" => Some(KeyDef {
            code: Cow::Borrowed("Home"),
            virtual_key: 36,
            insert_text: "",
        }),
        "End" => Some(KeyDef {
            code: Cow::Borrowed("End"),
            virtual_key: 35,
            insert_text: "",
        }),
        "PageUp" => Some(KeyDef {
            code: Cow::Borrowed("PageUp"),
            virtual_key: 33,
            insert_text: "",
        }),
        "PageDown" => Some(KeyDef {
            code: Cow::Borrowed("PageDown"),
            virtual_key: 34,
            insert_text: "",
        }),
        "Insert" => Some(KeyDef {
            code: Cow::Borrowed("Insert"),
            virtual_key: 45,
            insert_text: "",
        }),
        k if k.starts_with('F') && k.len() >= 2 => {
            let num: u32 = k[1..].parse().ok()?;
            if (1..=12).contains(&num) {
                Some(KeyDef {
                    code: Cow::Owned(k.to_string()),
                    virtual_key: 111 + num as i64,
                    insert_text: "",
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enter_key() {
        let def = get_key_def("Enter").unwrap();
        assert_eq!(def.code, "Enter");
        assert_eq!(def.virtual_key, 13);
        assert_eq!(def.insert_text, "\r");
    }

    #[test]
    fn test_return_alias() {
        let def = get_key_def("Return").unwrap();
        assert_eq!(def.code, "Enter");
        assert_eq!(def.virtual_key, 13);
    }

    #[test]
    fn test_tab_key() {
        let def = get_key_def("Tab").unwrap();
        assert_eq!(def.code, "Tab");
        assert_eq!(def.virtual_key, 9);
        assert_eq!(def.insert_text, "\t");
    }

    #[test]
    fn test_escape_has_no_text() {
        let def = get_key_def("Escape").unwrap();
        assert_eq!(def.virtual_key, 27);
        assert!(def.insert_text.is_empty());
    }

    #[test]
    fn test_arrow_keys() {
        assert_eq!(get_key_def("ArrowLeft").unwrap().virtual_key, 37);
        assert_eq!(get_key_def("ArrowUp").unwrap().virtual_key, 38);
        assert_eq!(get_key_def("ArrowRight").unwrap().virtual_key, 39);
        assert_eq!(get_key_def("ArrowDown").unwrap().virtual_key, 40);
    }

    #[test]
    fn test_f_keys() {
        assert_eq!(get_key_def("F1").unwrap().virtual_key, 112);
        assert_eq!(get_key_def("F12").unwrap().virtual_key, 123);
    }

    #[test]
    fn test_f13_returns_none() {
        assert!(get_key_def("F13").is_none());
    }

    #[test]
    fn test_unknown_key_returns_none() {
        assert!(get_key_def("SomethingRandom").is_none());
    }

    #[test]
    fn test_backspace_and_delete() {
        assert_eq!(get_key_def("Backspace").unwrap().virtual_key, 8);
        assert_eq!(get_key_def("Delete").unwrap().virtual_key, 46);
        assert!(get_key_def("Backspace").unwrap().insert_text.is_empty());
    }
}
