// memory.sv
module shared_memory_unit
    import accel_pkg::*;
(
    input  logic clk,
    input  logic rst_n,
    
    // メモリポートA（ベクトルデータ用）
    input  logic [5:0] addr_a,         // 64エントリ (16 vectors * 4 units)
    input  logic we_a,
    input  logic [VECTOR_WIDTH-1:0] data_in_a,
    output logic [VECTOR_WIDTH-1:0] data_out_a,
    
    // メモリポートB（行列データ用）
    input  logic [7:0] addr_b,         // 256エントリ (16x16 matrix)
    input  logic we_b,
    input  logic [1:0] data_in_b,      // 行列は2bit表現
    output logic [1:0] data_out_b,

    // メモリステータス
    output logic busy,
    output logic [1:0] error_status    // [1]: アドレス範囲エラー, [0]: アクセス競合
);
    // メモリインスタンス
    (* ram_style = "block" *) logic [VECTOR_WIDTH-1:0] vector_mem[64];
    (* ram_style = "block" *) logic [1:0] matrix_mem[256];

    // エラー検出ロジック
    logic addr_a_error, addr_b_error;
    assign addr_a_error = addr_a >= 64;
    assign addr_b_error = addr_b >= 256;

    // ベクトルメモリアクセス
    always_ff @(posedge clk) begin
        if (!rst_n) begin
            data_out_a <= '0;
            error_status[1] <= 1'b0;
        end
        else begin
            if (addr_a_error) begin
                error_status[1] <= 1'b1;
                data_out_a <= '0;
            end
            else begin
                error_status[1] <= 1'b0;
                if (we_a) begin
                    vector_mem[addr_a] <= data_in_a;
                    data_out_a <= data_in_a;  // write-through behavior
                end
                else begin
                    data_out_a <= vector_mem[addr_a];
                end
            end
        end
    end

    // 行列メモリアクセス
    always_ff @(posedge clk) begin
        if (!rst_n) begin
            data_out_b <= '0;
            error_status[0] <= 1'b0;
        end
        else begin
            if (addr_b_error) begin
                error_status[0] <= 1'b1;
                data_out_b <= '0;
            end
            else begin
                error_status[0] <= 1'b0;
                if (we_b) begin
                    matrix_mem[addr_b] <= data_in_b;
                    data_out_b <= data_in_b;  // write-through behavior
                end
                else begin
                    data_out_b <= matrix_mem[addr_b];
                end
            end
        end
    end

    // ビジー状態の管理
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            busy <= 1'b0;
        end
        else begin
            busy <= we_a || we_b || addr_a_error || addr_b_error;
        end
    end

    // synthesis translate_off
    // シミュレーション用の初期化と検証
    initial begin
        for (int i = 0; i < 64; i++) begin
            vector_mem[i] = '0;
        end
        for (int i = 0; i < 256; i++) begin
            matrix_mem[i] = '0;
        end
    end

    // メモリアクセス違反の検出
    always @(posedge clk) begin
        if (we_a && addr_a_error) begin
            $display("Warning: Vector memory address out of range: %0d", addr_a);
        end
        if (we_b && addr_b_error) begin
            $display("Warning: Matrix memory address out of range: %0d", addr_b);
        end
    end
    // synthesis translate_on

endmodule

// メモリアドレス生成モジュール
module memory_address_generator
    import accel_pkg::*;
(
    input  logic [1:0] unit_id,
    input  logic [3:0] vector_index,
    input  logic [3:0] matrix_row,
    input  logic [3:0] matrix_col,
    output logic [5:0] vector_addr,
    output logic [7:0] matrix_addr
);
    // ユニットIDとベクトルインデックスからベクトルアドレスを生成
    assign vector_addr = {unit_id, vector_index};

    // 行と列から行列アドレスを生成
    assign matrix_addr = {matrix_row, matrix_col};

endmodule