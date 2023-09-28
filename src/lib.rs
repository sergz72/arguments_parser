use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::io::{Error, ErrorKind};
use std::str::FromStr;

pub trait ValueHandler {
    fn parse_value(&self, value: &str) -> bool;
    fn requires_value(&self) -> bool;
    fn set_value(&self);
    fn value_type(&self) -> String;
}

pub struct IntParameter {
    value: Cell<isize>,
    validator: fn(isize) -> bool
}

impl IntParameter {
    pub fn new(value: isize, validator: fn(isize) -> bool) -> IntParameter {
        IntParameter { validator, value: Cell::new(value) }
    }

    pub fn get_value(&self) -> isize {
        self.value.get()
    }
}

impl ValueHandler for IntParameter {
    fn parse_value(&self, value: &str) -> bool {
        if let Ok(v) = isize::from_str(value).map_err(|e| Error::new(ErrorKind::InvalidInput, e)) {
            if (self.validator)(v) {
                self.value.set(v);
                return true;
            }
        }
        false
    }

    fn requires_value(&self) -> bool {
        return true;
    }

    fn set_value(&self) {
    }

    fn value_type(&self) -> String {
        return " int".to_string()
    }
}

pub struct StringParameter {
    value: RefCell<String>,
}

impl StringParameter {
    pub fn new(value: &str) -> StringParameter {
        StringParameter { value: RefCell::new(value.to_string()) }
    }

    pub fn get_value(&self) -> String {
        self.value.borrow().clone()
    }
}

impl ValueHandler for StringParameter {
    fn parse_value(&self, value: &str) -> bool {
        *self.value.borrow_mut() = value.to_string();
        true
    }

    fn requires_value(&self) -> bool {
        return true;
    }

    fn set_value(&self) {
    }

    fn value_type(&self) -> String {
        return " string".to_string()
    }
}

pub struct EnumParameter {
    values: HashSet<String>,
    value: RefCell<String>,
}

impl EnumParameter {
    pub fn new(values: Vec<String>, value: &str) -> EnumParameter {
        EnumParameter { values: values.into_iter().collect(), value: RefCell::new(value.to_string()) }
    }

    pub fn get_value(&self) -> String {
        self.value.borrow().clone()
    }
}

impl ValueHandler for EnumParameter {
    fn parse_value(&self, value: &str) -> bool {
        if self.values.contains(value) {
            *self.value.borrow_mut() = value.to_string();
            true
        } else {
            false
        }
    }

    fn requires_value(&self) -> bool {
        return true;
    }

    fn set_value(&self) {
    }

    fn value_type(&self) -> String {
        let mut t = " ".to_string();
        let mut first = true;
        for v in &self.values {
            if first {
                first = false;
            } else {
                t.push('|');
            }
            t.push_str(v.as_str());
        }
        return t
    }
}

pub struct BoolParameter {
    value: Cell<bool>,
}

impl BoolParameter {
    pub fn new() -> BoolParameter {
        BoolParameter { value: Cell::new(false) }
    }

    pub fn get_value(&self) -> bool {
        self.value.get()
    }
}

impl ValueHandler for BoolParameter {
    fn parse_value(&self, _value: &str) -> bool {
        // should not be called
        false
    }

    fn requires_value(&self) -> bool {
        return false;
    }

    fn set_value(&self) {
        self.value.set(true);
    }

    fn value_type(&self) -> String {
        return "".to_string()
    }
}

pub struct SizeParameter {
    value: Cell<isize>,
    validator: fn(isize) -> bool
}

impl SizeParameter {
    pub fn new(value: isize, validator: fn(isize) -> bool) -> SizeParameter {
        SizeParameter { value: Cell::new(value), validator }
    }

    pub fn get_value(&self) -> isize {
        self.value.get()
    }
}

impl ValueHandler for SizeParameter {
    fn parse_value(&self, value: &str) -> bool {
        if value.len() == 0 {
            return false
        }
        let multiplier = match value.chars().last().unwrap() {
            'M' => 1024 * 1024,
            'K' => 1024,
            'G' => 1024 * 1024 * 1024,
            _ => 1
        };
        if let Ok(size) = if multiplier == 1 {
            isize::from_str(value)
        } else {
            let mut chars = value.chars();
            chars.next_back();
            isize::from_str(chars.as_str())
        } {
            let msize = size * multiplier;
            if (self.validator)(msize) {
                self.value.set(msize);
                return true;
            }
        }
        false
    }

    fn requires_value(&self) -> bool {
        return true;
    }

    fn set_value(&self) {
    }

    fn value_type(&self) -> String {
        return " size".to_string()
    }
}

#[derive(Clone)]
pub struct Switch<'a> {
    name: String,
    switch: Option<char>,
    ext_switch: Option<String>,
    handler: &'a dyn ValueHandler,
}

impl<'a> Switch<'a> {
    pub fn new(name: &str, switch: Option<char>, ext_switch: Option<&str>, handler: &'a dyn ValueHandler) -> Switch<'a> {
        Switch {
            name: name.to_string(),
            switch,
            ext_switch: ext_switch.map(|s|s.to_string()),
            handler,
        }
    }

    fn to_string(&self) -> String {
        let mut result = "".to_string();
        if let Some(sw) = self.switch {
            result.push_str(format!(" -{}", sw).as_str());
            if let Some(sw) = &self.ext_switch {
                result.push_str(format!(" (or --{})", sw).as_str());
            }
        } else if let Some(sw) = &self.ext_switch {
            result.push_str(format!(" --{}", sw).as_str());
        }
        result.push_str(self.handler.value_type().as_str());
        result.push_str(" - ");
        result.push_str(self.name.as_str());
        result
    }

    fn parse_value(&self, value: &str) -> bool {
        self.handler.parse_value(value)
    }

    fn requires_value(&self) -> bool {
        self.handler.requires_value()
    }

    fn set_value(&self) {
        self.handler.set_value()
    }
}

pub struct Arguments<'a> {
    program_name: String,
    switch_map: HashMap<char, Switch<'a>>,
    ext_switch_map: HashMap<String, Switch<'a>>,
    other_arguments: Vec<String>,
    other_argument_names: Option<Vec<String>>,
}

impl<'a> Arguments<'a> {
    pub fn new(program_name: &str, switches: &[Switch<'a>], other_argument_names: Option<Vec<String>>) -> Arguments<'a> {
        let mut switch_map = HashMap::new();
        let mut ext_switch_map = HashMap::new();
        for switch in switches {
            if let Some(sw) = switch.switch {
                switch_map.insert(sw, switch.clone());
            }
            if let Some(sw) = &switch.ext_switch {
                ext_switch_map.insert(sw.clone(), switch.clone());
            }
        }
        Arguments {
            program_name: program_name.to_string(),
            switch_map,
            ext_switch_map,
            other_arguments: Vec::new(),
            other_argument_names
        }
    }

    pub fn usage(&self) {
        let mut usage = "Usage: ".to_string();
        usage.push_str(&self.program_name);
        if let Some(other_argument_names) = self.other_argument_names.as_ref() {
            for name in other_argument_names {
                usage.push_str((" ".to_string() + name.as_str()).as_str())
            }
        }
        usage.push('\n');
        for (_name, sw) in &self.switch_map {
            usage.push_str(sw.to_string().as_str());
            usage.push('\n');
        }
        for (_name, sw) in &self.ext_switch_map {
            usage.push_str(sw.to_string().as_str());
            usage.push('\n');
        }
        println!("{}", usage);
    }

    pub fn build(&mut self, args: Vec<String>) -> Result<(), Error> {
        let mut current_parameter: Option<&Switch> = None;
        for arg in args {
            if let Some(p) = current_parameter {
                if !p.parse_value(arg.as_str()) {
                   return Err(Error::new(ErrorKind::InvalidInput,
                                            format!("invalid {} value", p.name)))?;
                }
                current_parameter = None;
            } else {
                if arg.starts_with('-') {
                    if arg.starts_with("--") {
                        if arg.len() == 2 {
                            return Err(Error::new(ErrorKind::InvalidInput, "invalid ext_switch"));
                        }
                        if let Some(p) = self.ext_switch_map.get(&arg.chars().skip(2).collect::<String>()) {
                            if p.requires_value() {
                                current_parameter = Some(p);
                            } else {
                                p.set_value();
                            }
                        } else {
                            return Err(Error::new(ErrorKind::InvalidInput, "unknown ext switch"));
                        }
                    } else {
                        if arg.len() != 2 {
                            return Err(Error::new(ErrorKind::InvalidInput, "invalid switch"));
                        }
                        if let Some(p) = self.switch_map.get(&arg.chars().skip(1).next().unwrap()) {
                            if p.requires_value() {
                                current_parameter = Some(p);
                            } else {
                                p.set_value();
                            }
                        } else {
                            return Err(Error::new(ErrorKind::InvalidInput, "unknown switch"));
                        }
                    }
                } else {
                    self.other_arguments.push(arg.clone());
                }
            }
        }
        if current_parameter.is_some() {
            return Err(Error::new(ErrorKind::InvalidInput, "switch value expected"));
        }
        if let Some(other_argument_names) = self.other_argument_names.as_ref() {
            if other_argument_names.len() != self.other_arguments.len() {
                return Err(Error::new(ErrorKind::InvalidInput, "incorrect number of arguments"));
            }
        }
        Ok(())
    }

    pub fn get_other_arguments(&self) -> &Vec<String> {
        &self.other_arguments
    }
}

#[cfg(test)]
mod tests {
    use crate::{Arguments, BoolParameter, EnumParameter, IntParameter, SizeParameter, StringParameter, Switch};

    #[test]
    fn test_arguments_parser() {
        let port_parameter = IntParameter::new(6379, |v|v>0);
        let max_memory_parameter = SizeParameter::new(1024 * 1024 * 1024, |v|v>0);//1G
        let threads_parameter = IntParameter::new(4, |v|v>0);
        let verbose_parameter = BoolParameter::new();
        let string_parameter = StringParameter::new("init");
        let enum_parameter = EnumParameter::new(vec!["value".to_string()], "init");
        let switches = [
            Switch::new("port", Some('p'), None, &port_parameter),
            Switch::new("maximum_memory", Some('m'), None, &max_memory_parameter),
            Switch::new("threads", Some('t'), None, &threads_parameter),
            Switch::new("verbose", Some('v'), None, &verbose_parameter),
            Switch::new("test", None, Some("ss"), &string_parameter),
            Switch::new("test_enum", Some('e'), None, &enum_parameter),
        ];
        let mut arguments = Arguments::new("cache", &switches,
                                           Some(vec!["arg1".to_string(), "arg2".to_string()]));
        let result = arguments.build(vec![
            "-p".to_string(), "3333".to_string(),
            "-m".to_string(), "1M".to_string(),
            "-t".to_string(), "12".to_string(),
            "-v".to_string(),
            "--ss".to_string(), "test".to_string(),
            "-e".to_string(), "value".to_string(),
            "arg1".to_string(), "arg2".to_string()]);
        assert!(result.is_ok(), "{}", result.err().map(|e|e.to_string()).unwrap_or("".to_string()));
        assert_eq!(3333, port_parameter.get_value());
        assert_eq!(1024 * 1024, max_memory_parameter.get_value());
        assert_eq!(12, threads_parameter.get_value());
        assert_eq!(true, verbose_parameter.get_value());
        assert_eq!("test", string_parameter.get_value());
        assert_eq!("value", enum_parameter.get_value());
        assert_eq!(vec!["arg1".to_string(), "arg2".to_string()], arguments.get_other_arguments().clone());
    }
}