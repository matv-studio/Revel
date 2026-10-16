; Crochets de l'installateur Windows (NSIS).
;
; Revel s'installe pour tous les utilisateurs, dans « Programmes » (`installMode: perMachine`).
; Les nouveaux sites sont créés par défaut dans le dossier `Projects` à côté de l'application :
; l'installateur le crée et l'ouvre en modification au groupe « Utilisateurs »
; (S-1-5-32-545, héritage aux fichiers et dossiers créés dedans). Sans ce droit, Revel propose
; `Documents\Revel` à la place.

!macro NSIS_HOOK_POSTINSTALL
  CreateDirectory "$INSTDIR\Projects"
  Push $0
  nsExec::ExecToLog 'icacls "$INSTDIR\Projects" /grant *S-1-5-32-545:(OI)(CI)M'
  Pop $0
  Pop $0
!macroend

; Désinstallation : les sites restent. Le dossier `Projects` n'est retiré que s'il est vide.
!macro NSIS_HOOK_POSTUNINSTALL
  RMDir "$INSTDIR\Projects"
  RMDir "$INSTDIR"
!macroend
