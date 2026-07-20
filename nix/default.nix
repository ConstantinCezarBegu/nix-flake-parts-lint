{
  lib,
  rustPlatform,
  installShellFiles,
  pkg-config,
}:
rustPlatform.buildRustPackage {
  pname = "nix-lint";
  version = "0.1.0";
  src = lib.cleanSource ./.;
  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = [ installShellFiles pkg-config ];

  postInstall = ''
    installShellCompletion --cmd nix-lint --bash <($out/bin/nix-lint generate-completion bash)
    installShellCompletion --cmd nix-lint --zsh <($out/bin/nix-lint generate-completion zsh)
    installShellCompletion --cmd nix-lint --fish <($out/bin/nix-lint generate-completion fish)
  '';

  meta = with lib; {
    description = "AST-based linter for Nix Flake Parts projects";
    homepage = "https://github.com/ConstantinCezarBegu/nix-flake-parts-lint";
    license = licenses.mit;
    platforms = platforms.all;
  };
}
