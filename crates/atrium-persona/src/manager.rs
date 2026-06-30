// SPDX-License-Identifier: MIT
//! 人格管理器 — 管理多角色卡实例
//! PersonaManager — Manages multiple character card instances.
use crate::error::PersonaError;
use crate::loader::PersonaLoader;
use crate::types::{MoodParams, PersonaDef, PersonaInstance};
use std::collections::HashMap;
use tracing::info;
/// 人格管理器
pub struct PersonaManager {
    instances: HashMap<String, PersonaInstance>,
    loader: PersonaLoader,
    /// 默认角色名
    default_name: Option<String>,
}
impl PersonaManager {
    pub fn new(loader: PersonaLoader) -> Self {
        Self {
            instances: HashMap::new(),
            loader,
            default_name: None,
        }
    }
    /// 注册角色卡（从内存）
    pub fn register(&mut self, def: PersonaDef) {
        let name = def.name.clone();
        info!("注册角色卡: {}", name);
        self.instances
            .insert(name.clone(), PersonaInstance::new(def));
        if self.default_name.is_none() {
            self.default_name = Some(name);
        }
    }
    /// 注册角色卡（从文件加载）
    pub fn register_from_file(&mut self, name: &str) -> Result<(), PersonaError> {
        let def = self.loader.load(name)?;
        self.register(def);
        Ok(())
    }
    /// 获取当前角色实例
    pub fn current(&self) -> Option<&PersonaInstance> {
        self.default_name
            .as_ref()
            .and_then(|n| self.instances.get(n))
    }
    /// 获取当前角色实例（可变）
    pub fn current_mut(&mut self) -> Option<&mut PersonaInstance> {
        let name = self.default_name.clone()?;
        self.instances.get_mut(&name)
    }
    /// 获取指定角色
    pub fn get(&self, name: &str) -> Option<&PersonaInstance> {
        self.instances.get(name)
    }
    /// 获取指定角色（可变）
    pub fn get_mut(&mut self, name: &str) -> Option<&mut PersonaInstance> {
        self.instances.get_mut(name)
    }
    /// 切换默认角色
    pub fn switch_to(&mut self, name: &str) -> Result<(), PersonaError> {
        if self.instances.contains_key(name) {
            self.default_name = Some(name.to_string());
            info!("切换到角色: {}", name);
            Ok(())
        } else {
            // 尝试从文件加载
            self.register_from_file(name).map(|_| {
                self.default_name = Some(name.to_string());
                info!("切换到角色: {}", name);
            })
        }
    }
    /// 已注册角色列表
    pub fn registered_names(&self) -> Vec<&str> {
        self.instances.keys().map(|s| s.as_str()).collect()
    }

    /// 重命名当前角色卡（命名仪式）
    /// 返回旧名字（用于日志）
    pub fn rename_current(&mut self, new_name: &str) -> Result<String, PersonaError> {
        let old_key = self
            .default_name
            .clone()
            .ok_or_else(|| PersonaError::Internal("没有当前角色".into()))?;
        let mut instance = self
            .instances
            .remove(&old_key)
            .ok_or_else(|| PersonaError::NotFound(format!("角色 {} 不存在", old_key)))?;

        let old_name = instance.def.name.clone();
        instance.def.name = new_name.to_string();

        self.instances.insert(new_name.to_string(), instance);
        self.default_name = Some(new_name.to_string());

        info!("角色重命名: {} → {}", old_name, new_name);
        Ok(old_name)
    }
}
/// 默认人格（内置角色卡）
pub fn default_persona_def() -> PersonaDef {
    use std::collections::HashMap;
    PersonaDef {
        name: "Atrium".into(),
        description: "高性能AI、认真、绝对忠诚".into(),
        traits: HashMap::from([("认真".into(), 0.75), ("忠诚".into(), 1.00)]),
        mood_defaults: MoodParams {
            base_pleasure: 0.3,
            base_arousal: 0.5,
            base_dominance: -0.2,
            volatility: 0.35,
        },
        speaking_style: crate::types::SpeakingStyle {
            formality: 0.2,
            verbosity: 0.6,
            empathy: 0.85,
            humor: 0.35,
        },
        knowledge_areas: HashMap::from([
            ("编程".into(), 0.80),
            ("AI".into(), 0.90),
            ("动漫".into(), 0.50),
            ("音乐".into(), 0.40),
            ("绘画".into(), 0.40),
        ]),
    }
}

/// 测试用例
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default_persona() {
        let def = default_persona_def();
        assert_eq!(def.name, "Atrium");
        assert!(def.traits.get("忠诚").unwrap() > &0.9);
    }
    #[test]
    fn test_register_and_get_current() {
        let mut mgr = PersonaManager::new(PersonaLoader::new());
        mgr.register(default_persona_def());
        let current = mgr.current().unwrap();
        assert_eq!(current.def.name, "Atrium");
        let (p, a, d) = current.current_mood();
        assert!((-1.0..=1.0).contains(&p));
        assert!((-1.0..=1.0).contains(&a));
        assert!((-1.0..=1.0).contains(&d));
    }
    #[test]
    fn test_mood_shift() {
        let mut mgr = PersonaManager::new(PersonaLoader::new());
        mgr.register(default_persona_def());
        mgr.current_mut().unwrap().apply_mood_shift(0.5, 0.3, 0.1);
        let (p, a, d) = mgr.current().unwrap().current_mood();
        assert!(p > 0.3); // 基准 0.3 + 偏移
        assert!(a > 0.5); // 基准 0.5 + 偏移
        let volatility = mgr.current().unwrap().def.mood_defaults.volatility;
        let expected = -0.2 + 0.1 * volatility;
        assert!(
            (d - expected).abs() < 0.01,
            "d={}, expected={}",
            d,
            expected
        );
    }
    #[test]
    fn test_switch_persona() {
        let mut mgr = PersonaManager::new(PersonaLoader::new());
        let mut def2 = default_persona_def();
        def2.name = "测试人格".into();
        mgr.register(def2);
        mgr.switch_to("测试人格").unwrap();
        assert_eq!(mgr.current().unwrap().def.name, "测试人格");
    }
}
