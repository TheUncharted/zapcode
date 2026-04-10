use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::error::{Result, ZapcodeError};
use crate::sandbox::ResourceLimits;
use crate::value::Value;
use crate::vm::{CallFrame, Continuation, TryInfo, Vm, VmState};

/// Internal serializable representation of VM state at a suspension point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct VmSnapshot {
    pub(crate) programs: Vec<crate::compiler::CompiledProgram>,
    pub(crate) stack: Vec<Value>,
    pub(crate) frames: Vec<CallFrame>,
    /// User-defined globals only — builtins are re-registered on resume.
    pub(crate) globals: Vec<(String, Value)>,
    pub(crate) try_stack: Vec<TryInfo>,
    pub(crate) continuations: Vec<Continuation>,
    pub(crate) stdout: String,
    pub(crate) limits: ResourceLimits,
    pub(crate) external_functions: Vec<String>,
    pub(crate) next_generator_id: u64,
}

impl VmSnapshot {
    pub(crate) fn capture(vm: &Vm) -> Self {
        // Filter out builtin globals — they'll be re-registered on resume.
        let builtin_names: HashSet<&str> = Vm::BUILTIN_GLOBAL_NAMES.iter().copied().collect();
        let user_globals: Vec<(String, Value)> = vm
            .globals
            .iter()
            .filter(|(k, _)| !builtin_names.contains(k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Self {
            programs: vm.programs.clone(),
            stack: vm.stack.clone(),
            frames: vm.frames.clone(),
            globals: user_globals,
            try_stack: vm.try_stack.clone(),
            continuations: vm.continuations.clone(),
            stdout: vm.stdout.clone(),
            limits: vm.limits.clone(),
            external_functions: vm.external_functions.iter().cloned().collect(),
            next_generator_id: vm.next_generator_id,
        }
    }

    pub(crate) fn restore_vm(self) -> Vm {
        let user_globals: HashMap<String, Value> = self.globals.into_iter().collect();
        let ext_set: HashSet<String> = self.external_functions.into_iter().collect();

        Vm::from_snapshot(
            self.programs,
            self.stack,
            self.frames,
            user_globals,
            self.try_stack,
            self.continuations,
            self.stdout,
            self.limits,
            ext_set,
            self.next_generator_id,
        )
    }
}

/// A snapshot of VM state at a suspension point.
/// Can be serialized to bytes and resumed later in any process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapcodeSnapshot {
    data: Vec<u8>,
}

impl ZapcodeSnapshot {
    /// Capture the current VM state as a snapshot.
    pub(crate) fn capture(vm: &Vm) -> Result<Self> {
        let snapshot = VmSnapshot::capture(vm);

        let data = postcard::to_allocvec(&snapshot)
            .map_err(|e| ZapcodeError::SnapshotError(format!("capture failed: {}", e)))?;

        Ok(Self { data })
    }

    /// Serialize the snapshot to bytes for storage / transport.
    pub fn dump(&self) -> Result<Vec<u8>> {
        postcard::to_allocvec(self)
            .map_err(|e| ZapcodeError::SnapshotError(format!("dump failed: {}", e)))
    }

    /// Deserialize a snapshot from bytes.
    pub fn load(bytes: &[u8]) -> Result<Self> {
        postcard::from_bytes(bytes)
            .map_err(|e| ZapcodeError::SnapshotError(format!("load failed: {}", e)))
    }

    /// Resume execution with a return value from the external function.
    /// Returns a `VmState` which may be `Complete` or another `Suspended`.
    pub fn resume(self, return_value: Value) -> Result<VmState> {
        let vm_snap: VmSnapshot = postcard::from_bytes(&self.data)
            .map_err(|e| ZapcodeError::SnapshotError(format!("resume decode failed: {}", e)))?;

        let mut vm = vm_snap.restore_vm();

        // Push the return value onto the stack — this is the result the
        // `CallExternal` instruction was waiting for.
        vm.stack.push(return_value);

        vm.resume_execution()
    }
}
