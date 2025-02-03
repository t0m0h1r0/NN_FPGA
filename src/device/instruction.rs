/// VLIWコマンドの列挙型
#[derive(Debug, Clone, Copy)]
pub enum VliwCommand {
    Nop,
    LoadV0,
    LoadV1,
    LoadM0,
    StoreV0,
    StoreV1,
    StoreM0,
    ZeroV0,
    ZeroV1,
    ZeroM0,
    PushV0,
    PopV1,
    MatrixVectorMultiply,
    VectorAdd01,
    VectorSub01,
    VectorReLU,
    VectorTanh,
    VectorSquare,
}

/// VLIWインストラクション
#[derive(Debug, Clone)]
pub struct VliwInstruction {
    pub op1: VliwCommand,
    pub op2: VliwCommand,
    pub op3: VliwCommand,
    pub op4: VliwCommand,
}

impl VliwInstruction {
    pub fn nop() -> Self {
        Self {
            op1: VliwCommand::Nop,
            op2: VliwCommand::Nop,
            op3: VliwCommand::Nop,
            op4: VliwCommand::Nop,
        }
    }

    pub fn single(op: VliwCommand) -> Self {
        Self {
            op1: op,
            op2: VliwCommand::Nop,
            op3: VliwCommand::Nop,
            op4: VliwCommand::Nop,
        }
    }
}

/// VLIWインストラクションのビルダー
pub struct InstructionBuilder {
    inst: VliwInstruction,
    current_slot: usize,
}

impl InstructionBuilder {
    pub fn new() -> Self {
        Self {
            inst: VliwInstruction::nop(),
            current_slot: 0,
        }
    }

    pub fn add_op(&mut self, op: VliwCommand) -> &mut Self {
        if self.current_slot < 4 {
            match self.current_slot {
                0 => self.inst.op1 = op,
                1 => self.inst.op2 = op,
                2 => self.inst.op3 = op,
                3 => self.inst.op4 = op,
                _ => {}
            }
            self.current_slot += 1;
        }
        self
    }

    pub fn build(self) -> VliwInstruction {
        self.inst
    }
}