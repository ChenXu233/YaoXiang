import re, glob

# Shape 1: first-arg List/Dict handle destructure
ARG_LIST = re.compile(
    r"let ([a-z_]+) = match args\.first\(\) \{\n"
    r"        Some\(RuntimeValue::List\(h\)\) => \*h,\n"
    r"        _ => \{\n"
    r"            return Err\(ExecutorError::type_only\(\n"
    r'                "([a-z_.]+) expects a List as first argument"\.to_string\(\),\n'
    r"            \)\)\n"
    r"        \}\n"
    r"    \};",
    re.M)

ARG_DICT = re.compile(
    r"let ([a-z_]+) = match args\.first\(\) \{\n"
    r"        Some\(RuntimeValue::Dict\(h\)\) => \*h,\n"
    r"        _ => \{\n"
    r"            return Err\(ExecutorError::type_only\(\n"
    r'                "([a-z_.]+) expects a Dict as first argument"\.to_string\(\),\n'
    r"            \)\)\n"
    r"        \}\n"
    r"    \};",
    re.M)

# Shape 2a: heap.get list clone
HEAP_LIST = re.compile(
    r"match ctx\.heap\.get\(([a-z_]+)\) \{\n"
    r"        Some\(HeapValue::List\(items\)\) => items\.clone\(\),\n"
    r"        _ => \{\n"
    r"            return Err\(ExecutorError::runtime_only\(\n"
    r'                "Invalid list handle"\.to_string\(\),\n'
    r"            \)\)\n"
    r"        \}\n"
    r"    \}",
    re.M)

# Shape 2b: heap.get_mut list
HEAP_LIST_MUT = re.compile(
    r"match ctx\.heap\.get_mut\(([a-z_]+)\) \{\n"
    r"        Some\(HeapValue::List\(items\)\) => items,\n"
    r"        _ => \{\n"
    r"            return Err\(ExecutorError::runtime_only\(\n"
    r'                "Invalid list handle"\.to_string\(\),\n'
    r"            \)\)\n"
    r"        \}\n"
    r"    \}",
    re.M)

# Shape 2c: heap.get dict clone
HEAP_DICT = re.compile(
    r"match ctx\.heap\.get\(([a-z_]+)\) \{\n"
    r"        Some\(HeapValue::Dict\(map\)\) => map\.clone\(\),\n"
    r"        _ => \{\n"
    r"            return Err\(ExecutorError::runtime_only\(\n"
    r'                "Invalid dict handle"\.to_string\(\),\n'
    r"            \)\)\n"
    r"        \}\n"
    r"    \}",
    re.M)

# Shape 2d: heap.get_mut dict
HEAP_DICT_MUT = re.compile(
    r"match ctx\.heap\.get_mut\(([a-z_]+)\) \{\n"
    r"        Some\(HeapValue::Dict\(map\)\) => map,\n"
    r"        _ => \{\n"
    r"            return Err\(ExecutorError::runtime_only\(\n"
    r'                "Invalid dict handle"\.to_string\(\),\n'
    r"            \)\)\n"
    r"        \}\n"
    r"    \}",
    re.M)

total = {}
for path in glob.glob('src/std/*.rs'):
    s = open(path, encoding='utf-8').read()
    orig = s
    s, n1 = ARG_LIST.subn(lambda m: f"let {m.group(1)} = expect_list(args, \"{m.group(2)}\")?;", s)
    s, n2 = ARG_DICT.subn(lambda m: f"let {m.group(1)} = expect_dict(args, \"{m.group(2)}\")?;", s)
    s, n3 = HEAP_LIST.subn(lambda m: f"ctx.heap_list({m.group(1)})?", s)
    s, n4 = HEAP_LIST_MUT.subn(lambda m: f"ctx.heap_list_mut({m.group(1)})?", s)
    s, n5 = HEAP_DICT.subn(lambda m: f"ctx.heap_dict({m.group(1)})?", s)
    s, n6 = HEAP_DICT_MUT.subn(lambda m: f"ctx.heap_dict_mut({m.group(1)})?", s)
    n = n1 + n2 + n3 + n4 + n5 + n6
    if n:
        open(path, 'w', encoding='utf-8').write(s)
        total[path] = n
        print(f"{path}: list={n1} dict={n2} heap_list={n3} heap_list_mut={n4} heap_dict={n5} heap_dict_mut={n6}")
print("TOTAL:", sum(total.values()))
