{ pkgs, ... }:
{
  packages = [ pkgs.foundry pkgs.jq ];
  languages.javascript = {
    enable = true;
    package = pkgs.nodejs_22;
    yarn.enable = true;
  };
  languages.rust.enable = true;
}
