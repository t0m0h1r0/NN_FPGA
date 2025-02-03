# Makefile for Matrix Processor Simulation

# コンパイラ設定
COMPILER = iverilog
SIMULATOR = vvp
VIEWER = gtkwave

# ソースファイル
SRCS = mtx_types_pkg.sv shared_memory.sv mtx_unit.sv mtx_unit_tb.sv multi_unit_processor.sv

# 出力ファイル
TARGET = mtx_unit_tb
VCD_FILE = $(TARGET).vcd

# デフォルトターゲット
all: simulate

# シミュレーション用コンパイル
compile:
	@echo "コンパイル中..."
	$(COMPILER) -g2012 -o $(TARGET) $(SRCS)

# シミュレーション実行
simulate: compile
	@echo "シミュレーション実行中..."
	$(SIMULATOR) ./$(TARGET)

# 波形表示
view:
	$(VIEWER) $(VCD_FILE)

# クリーンアップ
clean:
	@echo "クリーンアップ中..."
	rm -f $(TARGET) $(VCD_FILE)

# 擬似ターゲット
.PHONY: all compile simulate view clean
