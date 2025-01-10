(module $call.wasm
  (type (;0;) (func))
  (type (;1;) (func (param i32) (result i32)))
  (func $__wasm_call_ctors (;0;) (type 0))
  (func $increment (;1;) (type 1) (param i32) (result i32)
    (local i32 i32 i32 i32 i32 i32)
    global.get $__stack_pointer
    local.set 1
    i32.const 16
    local.set 2
    local.get 1
    local.get 2
    i32.sub
    local.set 3
    local.get 3
    local.get 0
    i32.store offset=12
    local.get 3
    i32.load offset=12
    local.set 4
    i32.const 1
    local.set 5
    local.get 4
    local.get 5
    i32.add
    local.set 6
    local.get 6
    return
  )
  (func $triple (;2;) (type 1) (param i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $__stack_pointer
    local.set 1
    i32.const 16
    local.set 2
    local.get 1
    local.get 2
    i32.sub
    local.set 3
    local.get 3
    global.set $__stack_pointer
    local.get 3
    local.get 0
    i32.store offset=12
    local.get 3
    i32.load offset=12
    local.set 4
    i32.const 2
    local.set 5
    local.get 5
    call $increment
    local.set 6
    local.get 4
    local.get 6
    i32.mul
    local.set 7
    i32.const 16
    local.set 8
    local.get 3
    local.get 8
    i32.add
    local.set 9
    local.get 9
    global.set $__stack_pointer
    local.get 7
    return
  )
  (memory (;0;) 2)
  (global $__stack_pointer (;0;) (mut i32) i32.const 66560)
  (global (;1;) i32 i32.const 1024)
  (global (;2;) i32 i32.const 1024)
  (global (;3;) i32 i32.const 1024)
  (global (;4;) i32 i32.const 66560)
  (global (;5;) i32 i32.const 1024)
  (global (;6;) i32 i32.const 66560)
  (global (;7;) i32 i32.const 131072)
  (global (;8;) i32 i32.const 0)
  (global (;9;) i32 i32.const 1)
  (export "memory" (memory 0))
  (export "__wasm_call_ctors" (func $__wasm_call_ctors))
  (export "increment" (func $increment))
  (export "triple" (func $triple))
  (export "__dso_handle" (global 1))
  (export "__data_end" (global 2))
  (export "__stack_low" (global 3))
  (export "__stack_high" (global 4))
  (export "__global_base" (global 5))
  (export "__heap_base" (global 6))
  (export "__heap_end" (global 7))
  (export "__memory_base" (global 8))
  (export "__table_base" (global 9))
  (@producers
    (processed-by "Homebrew clang" "18.1.8")
  )
  (@custom "target_features" (after code) "\02+\0fmutable-globals+\08sign-ext")
)
