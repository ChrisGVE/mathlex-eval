public func ffi_compile<GenericToRustStr: ToRustStr>(_ ast_json: GenericToRustStr, _ constants_json: GenericToRustStr) throws -> RustCompiledExpr {
    return constants_json.toRustStr({ constants_jsonAsRustStr in
        return ast_json.toRustStr({ ast_jsonAsRustStr in
        try { let val = __swift_bridge__$ffi_compile(ast_jsonAsRustStr, constants_jsonAsRustStr); if val.is_ok { return RustCompiledExpr(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
    })
    })
}
public func ffi_eval_json<GenericToRustStr: ToRustStr>(_ expr: RustCompiledExprRef, _ args_json: GenericToRustStr) throws -> RustEvalHandle {
    return args_json.toRustStr({ args_jsonAsRustStr in
        try { let val = __swift_bridge__$ffi_eval_json(expr.ptr, args_jsonAsRustStr); if val.is_ok { return RustEvalHandle(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
    })
}
public func ffi_argument_names(_ expr: RustCompiledExprRef) -> RustVec<RustString> {
    RustVec(ptr: __swift_bridge__$ffi_argument_names(expr.ptr))
}
public func ffi_is_complex(_ expr: RustCompiledExprRef) -> Bool {
    __swift_bridge__$ffi_is_complex(expr.ptr)
}
public func ffi_shape(_ handle: RustEvalHandleRef) -> RustVec<Int64> {
    RustVec(ptr: __swift_bridge__$ffi_shape(handle.ptr))
}
public func ffi_len(_ handle: RustEvalHandleRef) -> Int64 {
    __swift_bridge__$ffi_len(handle.ptr)
}
public func ffi_scalar_json(_ handle: RustEvalHandle) throws -> RustString {
    try { let val = __swift_bridge__$ffi_scalar_json({handle.isOwned = false; return handle.ptr;}()); if val.is_ok { return RustString(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func ffi_to_array_json(_ handle: RustEvalHandle) throws -> RustString {
    try { let val = __swift_bridge__$ffi_to_array_json({handle.isOwned = false; return handle.ptr;}()); if val.is_ok { return RustString(ptr: val.ok_or_err!) } else { throw RustString(ptr: val.ok_or_err!) } }()
}
public func ffi_into_iter(_ handle: RustEvalHandle) -> RustEvalIter {
    RustEvalIter(ptr: __swift_bridge__$ffi_into_iter({handle.isOwned = false; return handle.ptr;}()))
}
public func ffi_iter_next(_ iter: RustEvalIterRefMut) -> Optional<RustString> {
    { let val = __swift_bridge__$ffi_iter_next(iter.ptr); if val != nil { return RustString(ptr: val!) } else { return nil } }()
}

public class RustEvalIter: RustEvalIterRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$RustEvalIter$_free(ptr)
        }
    }
}
public class RustEvalIterRefMut: RustEvalIterRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class RustEvalIterRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension RustEvalIter: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_RustEvalIter$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_RustEvalIter$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: RustEvalIter) {
        __swift_bridge__$Vec_RustEvalIter$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_RustEvalIter$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (RustEvalIter(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<RustEvalIterRef> {
        let pointer = __swift_bridge__$Vec_RustEvalIter$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return RustEvalIterRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<RustEvalIterRefMut> {
        let pointer = __swift_bridge__$Vec_RustEvalIter$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return RustEvalIterRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<RustEvalIterRef> {
        UnsafePointer<RustEvalIterRef>(OpaquePointer(__swift_bridge__$Vec_RustEvalIter$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_RustEvalIter$len(vecPtr)
    }
}


public class RustEvalHandle: RustEvalHandleRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$RustEvalHandle$_free(ptr)
        }
    }
}
public class RustEvalHandleRefMut: RustEvalHandleRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class RustEvalHandleRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension RustEvalHandle: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_RustEvalHandle$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_RustEvalHandle$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: RustEvalHandle) {
        __swift_bridge__$Vec_RustEvalHandle$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_RustEvalHandle$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (RustEvalHandle(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<RustEvalHandleRef> {
        let pointer = __swift_bridge__$Vec_RustEvalHandle$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return RustEvalHandleRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<RustEvalHandleRefMut> {
        let pointer = __swift_bridge__$Vec_RustEvalHandle$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return RustEvalHandleRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<RustEvalHandleRef> {
        UnsafePointer<RustEvalHandleRef>(OpaquePointer(__swift_bridge__$Vec_RustEvalHandle$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_RustEvalHandle$len(vecPtr)
    }
}


public class RustCompiledExpr: RustCompiledExprRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$RustCompiledExpr$_free(ptr)
        }
    }
}
public class RustCompiledExprRefMut: RustCompiledExprRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class RustCompiledExprRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension RustCompiledExpr: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_RustCompiledExpr$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_RustCompiledExpr$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: RustCompiledExpr) {
        __swift_bridge__$Vec_RustCompiledExpr$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_RustCompiledExpr$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (RustCompiledExpr(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<RustCompiledExprRef> {
        let pointer = __swift_bridge__$Vec_RustCompiledExpr$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return RustCompiledExprRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<RustCompiledExprRefMut> {
        let pointer = __swift_bridge__$Vec_RustCompiledExpr$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return RustCompiledExprRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<RustCompiledExprRef> {
        UnsafePointer<RustCompiledExprRef>(OpaquePointer(__swift_bridge__$Vec_RustCompiledExpr$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_RustCompiledExpr$len(vecPtr)
    }
}



