(module
  (type (;0;) (func (param i32)))
  (type (;1;) (func))
  (import "env" "puts" (func (;0;) (type 0)))
  (memory (;0;) 2)
  (global (;0;) i32 i32.const 1024)
  (global (;1;) i32 i32.const 1035)
  (global (;2;) i32 i32.const 1040)
  (global (;3;) i32 i32.const 66576)
  (global (;4;) i32 i32.const 1024)
  (global (;5;) i32 i32.const 66576)
  (global (;6;) i32 i32.const 131072)
  (global (;7;) i32 i32.const 0)
  (global (;8;) i32 i32.const 1)
  (export "memory" (memory 0))
  (export "__wasm_call_ctors" (func 1))
  (export "print_is_even" (func 2))
  (export "__dso_handle" (global 0))
  (export "__data_end" (global 1))
  (export "__stack_low" (global 2))
  (export "__stack_high" (global 3))
  (export "__global_base" (global 4))
  (export "__heap_base" (global 5))
  (export "__heap_end" (global 6))
  (export "__memory_base" (global 7))
  (export "__table_base" (global 8))
  (func (;1;) (type 1))
  (func (;2;) (type 0) (param i32)
    local.get 0
    i32.const 2
    i32.rem_s
    i32.eqz
    if
      i32.const 1024
    else
      i32.const 1030
    end
    call 0
  )
  (data (;0;) (i32.const 1024) "even!\00odd!")
  (@producers
    (processed-by "Homebrew clang" "20.1.5")
  )
  (@custom "target_features" (after data) "\06+\0fmutable-globals+\13nontrapping-fptoint+\0bbulk-memory+\08sign-ext+\0freference-types+\0amultivalue")
)
