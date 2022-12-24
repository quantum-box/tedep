{{/*
Create controller name and version as used by the chart label.
Truncated at 52 chars because Deployment label 'controller-revision-hash' is limited
to 63 chars and it includes 10 chars of hash and a separating '-'.
*/}}
{{- define "tedep.controller.fullname" -}}
{{- printf "%s-%s" (include "tedep.name" .) .Values.controller.name | trunc 52 | trimSuffix "-" -}}
{{- end -}}
