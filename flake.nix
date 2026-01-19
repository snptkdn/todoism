{
  description = "Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Rustのツールチェーン定義（ここでバージョンやtargetを指定）
        # 例: Stableの最新版、かつrust-analyzerを含む
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };
        
        # 特定のバージョンを使いたい場合
        # rustToolchain = pkgs.rust-bin.stable."1.75.0".default.override { ... };
        
        # rust-toolchain.toml ファイルがある場合
        # rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config    # C言語ライブラリの依存解決に必須
            openssl       # 多くのRustクレートで必要になる
          ];

          # Rust Analyzerなどがソースコードを見つけられるようにする
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          
          # C言語ライブラリのリンク問題を解決するための環境変数
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
             pkgs.openssl
             # 他に必要な .so ファイルがあればここに追加
          ];
        };
      }
    );
}
