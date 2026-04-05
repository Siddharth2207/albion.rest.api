rec {
  keys = {
    alastair =
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJArH3PA+bFIon0JkCVQGs9aWr45lnVjiiTLLO9BPItn";
    github_actions_deploy =
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIN8tXytd8vWClKbJ+xSyCFNHlIaR4R4KGOb9IUGaxSlk";
    host =
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICMHgH+tMB9/MWcKQU5U07I02eMZmPrVvY8oSphg/1lp";
  };

  roles = with keys; {
    infra = [ alastair github_actions_deploy ];
    ssh = [ alastair github_actions_deploy host ];
  };
}
