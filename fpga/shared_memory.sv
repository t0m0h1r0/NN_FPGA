`ifndef SHARED_MEMORY_MODULE
`define SHARED_MEMORY_MODULE

`include "mtx_types_pkg.sv"

module shared_memory(
    input logic clk,
    input logic rst_n,
    
    // ライト/リードポート
    input logic [4:0] write_unit_id,   // 書き込み元ユニットID
    input logic [4:0] read_unit_id,    // 読み出し先ユニットID
    input mtx_types::mv_t write_data,  // 書き込みデータ
    input logic write_enable,
    
    // 読み出しデータ
    output mtx_types::mv_t read_data
);
    import mtx_types::*;

    // ユニット数に応じた共有メモリ
    // 各ユニットに1エントリ分の領域
    mv_t [31:0] memory;

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            // 初期化
            memory <= '{default: '0};
        end else if (write_enable) begin
            // 指定されたユニットIDの領域に書き込み
            memory[write_unit_id] <= write_data;
        end
    end

    // 読み出し（組み合わせロジック）
    assign read_data = memory[read_unit_id];
endmodule

`endif
