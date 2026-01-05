# Task 15.2: 集合类型

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

提供 List、Map、Set、Queue、Stack 等集合类型。

## 集合类型

```yaoxiang
# List - 可变长数组
List[T] = struct {
    elements: Array[T],
    length: Int,
}

# Map - 键值对映射
Map[K, V] = struct {
    buckets: Array[List[(K, V)]>,
    size: Int,
}

# Set - 集合
Set[T] = Map[T, Bool]

# Queue - 队列
Queue[T] = struct {
    elements: List[T],
    front: Int,
}

# Stack - 栈
Stack[T] = struct {
    elements: List[T],
}
```

## List API

```yaoxiang
# List 方法
list = List::new()
list = list.push(1)
list = list.push(2)
list = list.push(3)
assert(list.length == 3)
assert(list[0] == 1)
assert(list.pop() == 3)

# 遍历
list.for_each(|x| print(x))
mapped = list.map(|x| x * 2)
filtered = list.filter(|x| x > 1)
```

## 验收测试

```yaoxiang
# test_collections.yx

# List
list = [1, 2, 3, 4, 5]
assert(list.length == 5)
assert(list.sum() == 15)
assert(list.contains(3))

# Map
scores = Map::new()
scores["alice"] = 90
scores["bob"] = 85
assert(scores["alice"] == 90)

# Set
set = Set::from([1, 2, 3])
assert(set.contains(2))

print("Collections tests passed!")
```

## 相关文件

- **collections/mod.rs**
- **collections/list.rs**
- **collections/map.rs**
