{{/*
Expand the name of the chart.
*/}}
{{- define "tedep.name" -}}
{{- default .Chart.Name .Values.name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Chart label
*/}}
{{- define "tedep.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Common labels
*/}}
{{- define "tedep.labels" -}}
helm.sh/chart: {{ include "tedep.chart" .context }}
{{ include "tedep.selectorLabels" (dict "context" .context "component" .component "name" .name) }}
app.kubernetes.io/managed-by: {{ .context.Release.Service }}
app.kubernetes.io/part-of: {{ include "tedep.name" .context }}
{{- with .context.Values.global.labels }}
{{ toYaml . }}
{{- end }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "tedep.selectorLabels" -}}
{{- if .name -}}
app.kuberentes.io/name: {{ include "tedep.name" .context -}}-{{- .name }}
{{ end -}}
app.kuberentes.io/instance: {{ .context.Release.Name }}
{{- if .component }}
app.kuberentes.io/component: {{ .component }}
{{- end }}
{{- end }}
