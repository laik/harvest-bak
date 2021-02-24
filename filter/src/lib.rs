use common::Item;
use regex::Regex;
use std::collections::HashMap;

pub struct Filter {
    json_filter_rules: HashMap<String, Regex>,
    default_filter_rules: Vec<Regex>,
}

impl Filter {
    pub fn new() -> Self {
        Filter {
            json_filter_rules: HashMap::new(),
            default_filter_rules: Vec::new(),
        }
    }

    pub fn add_json_rule(&mut self, key: &str, regex: Regex) -> &mut Self {
        self.json_filter_rules.insert(key.to_owned(), regex);
        self
    }

    pub fn add_default_rule(&mut self, regex: Regex) -> &mut Self {
        self.default_filter_rules.push(regex);
        self
    }

    pub fn get_default_rule(&self, key: &str) -> Option<&Regex> {
        self.default_filter_rules
            .iter()
            .find(|item| *item.as_str() == *key)
    }

    pub fn get_json_rule(&self, key: &str) -> Option<&Regex> {
        self.json_filter_rules.get(key)
    }

    pub fn get_json_rules(&self) -> Option<&HashMap<String, Regex>> {
        Some(&self.json_filter_rules)
    }

    pub fn remove_json_rule(&mut self, key: &str) -> &mut Self {
        self.json_filter_rules.remove(key);
        self
    }

    pub fn remove_line_rule(&mut self, regex: Regex) -> &mut Self {
        match self
            .default_filter_rules
            .iter()
            .position(|item| format!("{}", regex) == format!("{}", item))
        {
            None => self,
            Some(index) => {
                self.default_filter_rules.remove(index);
                self
            }
        }
    }

    pub fn pass(&self, item: &Item) -> bool {
        match item {
            Item::JSON(ref value) => {
                let obj = match value.as_object() {
                    None => {
                        return false;
                    }
                    Some(obj) => obj,
                };

                if let Some(rules) = self.get_json_rules() {
                    for (key, regex) in rules {
                        let is_match = match obj.get(&key.clone()) {
                            None => false,
                            Some(obj_v) => match obj_v.as_str() {
                                Some(v) => regex.is_match(v),
                                _ => false,
                            },
                        };
                        if !is_match {
                            return false;
                        }
                    }
                    return true;
                }

                false
            }

            Item::Default(ref line) => {
                self.default_filter_rules.
                    iter().
                    map(|reg| reg.is_match(&line)).
                    filter(|item| *item). // filter match line condition
                    collect::<Vec<_>>().len()
                    > 0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn regexp_match_test() {
        let rule = r#"hello"#;
        assert_eq!(false, Regex::new(rule).unwrap().is_match("helllo"));
    }

    #[test]
    fn filter_impl_works() {
        let mut filter = Filter::new();

        let rule = match Regex::new(r#"hello"#) {
            Ok(reg) => reg,
            Err(e) => {
                panic!("{}", e)
            }
        };

        filter.add_json_rule("pod_name", rule.clone());

        let reg = filter.get_json_rule("pod_name").unwrap();
        assert_eq!(format!("{}", rule), format!("{}", *reg)); // via regex Display trait

        assert_eq!(filter.pass(&Item::from(r#"{"pod_name":"hello"}"#)), true);
        assert_eq!(filter.pass(&Item::from(r#"{"pod_name":"world"}"#)), false);

        filter.remove_json_rule("pod_name");

        assert_eq!(filter.json_filter_rules.len(), 0);

        let rule = match Regex::new(r#"^w.*$"#) {
            Ok(reg) => reg,
            Err(e) => {
                panic!("{}", e)
            }
        };

        filter.add_json_rule("pod_name", rule);
        assert_eq!(filter.pass(&Item::from(r#"{"pod_name":"world"}"#)), true);
        assert_eq!(filter.pass(&Item::from(r#"{"pod_name":"123"}"#)), false);

        let line_rule = match Regex::new(r#"^w.*$"#) {
            Ok(reg) => reg,
            Err(e) => {
                panic!("{}", e)
            }
        };

        filter.add_default_rule(line_rule.clone());

        assert_eq!(filter.pass(&Item::from(r#"world"#)), true);

        filter.remove_line_rule(line_rule);
        assert_eq!(filter.default_filter_rules.len(), 0);
    }
}
