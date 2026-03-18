rec {
  keys = {
    albion-op =
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFXXJvJgZ9eUqD7ssxswi0GNFTfIXsfeHhntoUOkNFNI";

    host =
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFyhLQMjMcFnDiRJJ3dX5cN9SbJ919Rgaw2+8hMwsFOV";

    ci =
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPTd2zKSwHgWegi290EiK5nYp1Wp4+x2fDYqFxbd0WLN";

    arda =
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAyTREGZCOzMsl7N9dp1saN/t7DCs7YesusVUKApMJ78";
  };

  roles = with keys; {
    infra = [ ci albion-op ];
    ssh   = [ ci arda albion-op ];
  };
}