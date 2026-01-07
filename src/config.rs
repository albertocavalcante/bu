use starlark::environment::{GlobalsBuilder, Module, LibraryExtension};
use starlark::eval::Evaluator;
use starlark::syntax::{AstModule, Dialect};
use starlark::values::none::NoneType;
use starlark::values::Value;
use starlark::values::list::ListRef;
use starlark::starlark_module;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use anyhow::Result;
use crate::toolchain::{ToolProvider, UrlProvider, HostProvider, CargoBuildProvider, ChainProvider};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ToolDefinition {
    pub name: String,
    pub version: String,
    pub url_template: Option<String>,
    pub sha256: Option<String>,
    pub git_url: Option<String>,
    pub strategies: Vec<String>,
}

#[derive(Default)]
pub struct Config {
    pub tools: HashMap<String, ToolDefinition>,
}

thread_local! {
    static CONFIG_CAPTURE: RefCell<Option<Rc<RefCell<Config>>>> = const { RefCell::new(None) };
}

#[starlark_module]
fn bu_globals(builder: &mut GlobalsBuilder) {
    fn register_tool(name: String, 
                     version: String, 
                     url_template: Option<String>, 
                     sha256: Option<String>,
                     git_url: Option<String>,
                     strategies: Option<Value>) -> anyhow::Result<NoneType> {
        
        let strategies_vec = if let Some(v) = strategies {
            if let Some(list) = ListRef::from_value(v) {
                list.iter().map(|item| item.to_str()).collect()
            } else {
                return Err(anyhow::anyhow!("strategies must be a list of strings"));
            }
        } else {
             vec!["host".into(), "url".into()]
        };

        CONFIG_CAPTURE.with(|capture| {
            if let Some(config_rc) = capture.borrow().as_ref() {
                let def = ToolDefinition {
                    name: name.clone(),
                    version,
                    url_template,
                    sha256,
                    git_url,
                    strategies: strategies_vec,
                };
                config_rc.borrow_mut().tools.insert(name, def);
            }
        });
        
        Ok(NoneType)
    }
}

pub fn load_config(content: &str) -> Result<Config> {
    let config = Rc::new(RefCell::new(Config::default()));
    
    // Set thread local
    CONFIG_CAPTURE.with(|capture| {
        *capture.borrow_mut() = Some(config.clone());
    });

    // Use extended globals which includes 'struct' (StructType)
    let mut globals = GlobalsBuilder::extended_by(&[LibraryExtension::StructType]);
    bu_globals(&mut globals); // This calls the generated function
    
    let module = Module::new();
    let globals = globals.build();
    let mut evaluator = Evaluator::new(&module);
    
    // Preamble to alias
    let preamble = "bu = struct(register_tool = register_tool)";
    let preamble_ast = AstModule::parse("preamble.star", preamble.to_owned(), &Dialect::Standard)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    
    evaluator.eval_module(preamble_ast, &globals)
        .map_err(|e| anyhow::anyhow!("Preamble error: {}", e))?;

    // User content
    let ast = AstModule::parse("config.star", content.to_owned(), &Dialect::Standard)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
        
    let res = evaluator.eval_module(ast, &globals);

    // Clear thread local
    CONFIG_CAPTURE.with(|capture| {
        *capture.borrow_mut() = None;
    });

    res.map_err(|e| anyhow::anyhow!("{}", e))?;

    let result = config.borrow().tools.clone();
    Ok(Config { tools: result })
}

impl Config {
    pub fn get_tool_provider(&self, tool_name: &str) -> Option<Box<dyn ToolProvider>> {
        let def = self.tools.get(tool_name)?;
        
        let mut providers: Vec<Box<dyn ToolProvider>> = Vec::new();
        
        for strategy in &def.strategies {
            match strategy.as_str() {
                "host" => providers.push(Box::new(HostProvider)),
                "url" => {
                    if let Some(template) = &def.url_template {
                        providers.push(Box::new(UrlProvider {
                            url_template: template.clone(),
                            sha256: def.sha256.clone(),
                        }));
                    }
                }
                "source" => {
                    if let Some(git) = &def.git_url {
                        providers.push(Box::new(CargoBuildProvider {
                            git_url: git.clone(),
                            bin_name: tool_name.to_string(),
                        }));
                    }
                }
                _ => {}
            }
        }
        
        Some(Box::new(ChainProvider::new(providers)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starlark_config_loading() {
        let content = r#"
bu.register_tool(
    name = "buck2",
    version = "2024-01-01",
    url_template = "https://example.com/buck2",
    strategies = ["url", "host"]
)
"#;
        let config = load_config(content).unwrap();
        assert!(config.tools.contains_key("buck2"));
        
        let def = config.tools.get("buck2").unwrap();
        assert_eq!(def.version, "2024-01-01");
        assert_eq!(def.strategies, vec!["url", "host"]);
    }
}