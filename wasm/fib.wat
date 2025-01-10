(module
  (type (;0;) (func))
  (type (;1;) (func (param i32) (result i32)))
  (func (;0;) (type 0))
  (func (;1;) (type 1) (param i32) (result i32)
    (local i32)
    i32.const 2
    local.set 1
    local.get 0
    i32.const 3
    i32.ge_s
    if (result i32) ;; label = @1
      local.get 0
      i32.const 2
      i32.add
      local.set 0
      i32.const 0
      local.set 1
      loop ;; label = @2
        local.get 0
        i32.const 3
        i32.sub
        call 1
        local.get 1
        i32.add
        local.set 1
        local.get 0
        i32.const 2
        i32.sub
        local.tee 0
        i32.const 4
        i32.gt_u
        br_if 0 (;@2;)
      end
      local.get 1
      i32.const 2
      i32.add
    else
      i32.const 2
    end
  )
  (memory (;0;) 2)
  (global (;0;) i32 i32.const 1024)
  (global (;1;) i32 i32.const 1024)
  (global (;2;) i32 i32.const 1024)
  (global (;3;) i32 i32.const 66560)
  (global (;4;) i32 i32.const 1024)
  (global (;5;) i32 i32.const 66560)
  (global (;6;) i32 i32.const 131072)
  (global (;7;) i32 i32.const 0)
  (global (;8;) i32 i32.const 1)
  (export "memory" (memory 0))
  (export "__wasm_call_ctors" (func 0))
  (export "fib" (func 1))
  (export "__dso_handle" (global 0))
  (export "__data_end" (global 1))
  (export "__stack_low" (global 2))
  (export "__stack_high" (global 3))
  (export "__global_base" (global 4))
  (export "__heap_base" (global 5))
  (export "__heap_end" (global 6))
  (export "__memory_base" (global 7))
  (export "__table_base" (global 8))
  (@producers
    (processed-by "Homebrew clang" "18.1.8")
  )
  (@custom "target_features" (after code) "\02+\0fmutable-globals+\08sign-ext")
)
