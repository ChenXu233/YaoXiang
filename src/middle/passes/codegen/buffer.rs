//! 字节码缓冲区管理
//!
//! 管理常量池和字节码生成缓冲区。

use crate::middle::core::ir::ConstValue;

/// 常量池
#[derive(Debug, Default, Clone)]
pub struct ConstantPool {
    /// 常量列表
    constants: Vec<ConstValue>,
}

impl ConstantPool {
    /// 创建新常量池
    pub fn new() -> Self {
        ConstantPool {
            constants: Vec::new(),
        }
    }

    /// 添加常量并返回索引
    pub fn add(
        &mut self,
        value: ConstValue,
    ) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    /// 获取常量
    pub fn get(
        &self,
        index: usize,
    ) -> Option<&ConstValue> {
        self.constants.get(index)
    }

    /// 构建常量池
    pub fn build(self) -> Vec<ConstValue> {
        self.constants
    }

    /// 获取常量数量
    pub fn len(&self) -> usize {
        self.constants.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.constants.is_empty()
    }
}

/// 字节码缓冲区
///
/// 管理常量池和字节码生成的缓冲区。
#[derive(Debug, Default)]
pub struct BytecodeBuffer {
    /// 常量池
    constant_pool: ConstantPool,
    /// 字节码缓冲区
    bytecode: Vec<u8>,
}

impl BytecodeBuffer {
    /// 创建新的字节码缓冲区
    pub fn new() -> Self {
        BytecodeBuffer {
            constant_pool: ConstantPool::new(),
            bytecode: Vec::new(),
        }
    }

    /// 添加常量并返回索引
    pub fn add_constant(
        &mut self,
        value: ConstValue,
    ) -> usize {
        self.constant_pool.add(value)
    }

    /// 获取常量
    pub fn get_constant(
        &self,
        index: usize,
    ) -> Option<&ConstValue> {
        self.constant_pool.get(index)
    }

    /// 发射字节码指令
    pub fn emit(
        &mut self,
        bytes: &[u8],
    ) {
        self.bytecode.extend_from_slice(bytes);
    }

    /// 获取字节码内容
    pub fn bytecode(&self) -> &[u8] {
        &self.bytecode
    }

    /// 获取字节码内容（可变引用）
    pub fn bytecode_mut(&mut self) -> &mut Vec<u8> {
        &mut self.bytecode
    }

    /// 获取常量池
    pub fn into_constant_pool(self) -> ConstantPool {
        self.constant_pool
    }

    /// 获取常量池引用
    pub fn constant_pool(&self) -> &ConstantPool {
        &self.constant_pool
    }

    /// 获取常量池可变引用
    pub fn constant_pool_mut(&mut self) -> &mut ConstantPool {
        &mut self.constant_pool
    }

    /// 获取常量池数据（获取所有权并清空）
    pub fn take_constant_pool(&mut self) -> Vec<ConstValue> {
        std::mem::take(&mut self.constant_pool.constants)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_pool_add_get() {
        let mut pool = ConstantPool::new();
        let idx1 = pool.add(ConstValue::Int(42));
        let idx2 = pool.add(ConstValue::String("hello".to_string()));

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);

        assert_eq!(pool.get(0), Some(&ConstValue::Int(42)));
        assert_eq!(pool.get(1), Some(&ConstValue::String("hello".to_string())));
        assert_eq!(pool.get(2), None);
    }

    #[test]
    fn test_bytecode_buffer() {
        let mut buffer = BytecodeBuffer::new();

        buffer.emit(&[0x01, 0x02, 0x03]);
        buffer.emit(&[0x04, 0x05]);

        assert_eq!(buffer.bytecode(), &[0x01, 0x02, 0x03, 0x04, 0x05]);

        let idx = buffer.add_constant(ConstValue::Int(100));
        assert_eq!(idx, 0);
        assert_eq!(buffer.get_constant(0), Some(&ConstValue::Int(100)));
    }
}
