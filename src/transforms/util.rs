use super::super::prelude::*;

extern "C" {
    pub fn LLVMAddLowerSwitchPass(PM: LLVMPassManagerRef);

    pub fn LLVMAddPromoteMemoryToRegisterPass(PM: LLVMPassManagerRef);

    #[cfg(LLVM_VERSION_9_OR_GREATER)]
    pub fn LLVMAddAddDiscriminatorsPass(PM: LLVMPassManagerRef);
}
