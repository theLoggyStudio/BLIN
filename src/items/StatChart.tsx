import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Legend,
  Line,
  LineChart,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { Text } from "@/items/Text";
import { cn } from "@/lib/utils";

export type StatChartType = "bar" | "line" | "area" | "pie";

export interface StatChartDatum {
  label: string;
  value: number;
}

export interface StatChartSeriesDef {
  key: string;
  name: string;
  color: string;
}

export interface StatChartMultiDatum {
  label: string;
  [seriesKey: string]: string | number;
}

const PIE_COLORS = ["#4DB6AC", "#2563eb", "#dc2626", "#f59e0b", "#8b5cf6", "#ec4899"];

interface StatChartProps {
  title?: string;
  type?: StatChartType;
  /** Mode simple (une série) */
  data?: StatChartDatum[];
  /** Mode comparaison multi-entités */
  multiData?: StatChartMultiDatum[];
  series?: StatChartSeriesDef[];
  xLabel?: string;
  yLabel?: string;
  height?: number;
  className?: string;
}

function tooltipStyle() {
  return {
    contentStyle: { background: "#1e1e1e", border: "1px solid #404040", borderRadius: 8 },
    labelStyle: { color: "#fafafa" },
  };
}

/** Graphique statistique (barres, courbes, aires, secteurs, multi-séries). */
export function StatChart({
  title,
  type = "bar",
  data,
  multiData,
  series = [],
  xLabel = "Abscisse",
  yLabel = "Ordonnée",
  height = 300,
  className,
}: StatChartProps) {
  const isMulti = series.length > 0 && multiData && multiData.length > 0;
  const simpleRows =
    !isMulti && data
      ? data.map((d) => ({ name: d.label, value: d.value }))
      : [];
  const multiRows =
    isMulti && multiData
      ? multiData.map((d) => ({ name: d.label, ...d }))
      : [];

  const chart = (() => {
    if (isMulti && type === "pie") {
      const first = series[0];
      const pieRows = multiRows.map((r) => ({
        name: String(r.name),
        value: Number(r[first.key] ?? 0),
      }));
      return (
        <PieChart>
          <Pie data={pieRows} dataKey="value" nameKey="name" cx="50%" cy="50%" outerRadius="70%" label>
            {pieRows.map((_, i) => (
              <Cell key={i} fill={PIE_COLORS[i % PIE_COLORS.length]} />
            ))}
          </Pie>
          <Tooltip {...tooltipStyle()} />
          <Legend />
        </PieChart>
      );
    }

    if (isMulti) {
      const grid = <CartesianGrid strokeDasharray="3 3" stroke="#333" />;
      const axes = (
        <>
          <XAxis dataKey="name" stroke="#a3a3a3" tick={{ fontSize: 11 }} />
          <YAxis stroke="#a3a3a3" tick={{ fontSize: 11 }} />
          <Tooltip {...tooltipStyle()} />
          <Legend />
        </>
      );
      if (type === "line") {
        return (
          <LineChart data={multiRows}>
            {grid}
            {axes}
            {series.map((s) => (
              <Line
                key={s.key}
                type="monotone"
                dataKey={s.key}
                name={s.name}
                stroke={s.color}
                strokeWidth={2}
                dot={{ r: 3 }}
              />
            ))}
          </LineChart>
        );
      }
      if (type === "area") {
        return (
          <AreaChart data={multiRows}>
            {grid}
            {axes}
            {series.map((s) => (
              <Area
                key={s.key}
                type="monotone"
                dataKey={s.key}
                name={s.name}
                stroke={s.color}
                fill={`${s.color}33`}
              />
            ))}
          </AreaChart>
        );
      }
      return (
        <BarChart data={multiRows} barSize="28%">
          {grid}
          {axes}
          {series.map((s) => (
            <Bar key={s.key} dataKey={s.key} name={s.name} fill={s.color} radius={[4, 4, 0, 0]} />
          ))}
        </BarChart>
      );
    }

    switch (type) {
      case "line":
        return (
          <LineChart data={simpleRows}>
            <CartesianGrid strokeDasharray="3 3" stroke="#333" />
            <XAxis dataKey="name" stroke="#a3a3a3" tick={{ fontSize: 11 }} />
            <YAxis stroke="#a3a3a3" tick={{ fontSize: 11 }} />
            <Tooltip {...tooltipStyle()} />
            <Legend />
            <Line type="monotone" dataKey="value" stroke="#4DB6AC" strokeWidth={2} dot={{ r: 3 }} />
          </LineChart>
        );
      case "area":
        return (
          <AreaChart data={simpleRows}>
            <CartesianGrid strokeDasharray="3 3" stroke="#333" />
            <XAxis dataKey="name" stroke="#a3a3a3" />
            <YAxis stroke="#a3a3a3" />
            <Tooltip {...tooltipStyle()} />
            <Area type="monotone" dataKey="value" stroke="#4DB6AC" fill="#4DB6AC33" />
          </AreaChart>
        );
      case "pie":
        return (
          <PieChart>
            <Pie data={simpleRows} dataKey="value" nameKey="name" cx="50%" cy="50%" outerRadius="70%" label>
              {simpleRows.map((_, i) => (
                <Cell key={i} fill={PIE_COLORS[i % PIE_COLORS.length]} />
              ))}
            </Pie>
            <Tooltip {...tooltipStyle()} />
            <Legend />
          </PieChart>
        );
      default:
        return (
          <BarChart data={simpleRows} barSize="40%">
            <CartesianGrid strokeDasharray="3 3" stroke="#333" />
            <XAxis dataKey="name" stroke="#a3a3a3" tick={{ fontSize: 11 }} />
            <YAxis stroke="#a3a3a3" tick={{ fontSize: 11 }} />
            <Tooltip {...tooltipStyle()} />
            <Legend />
            <Bar dataKey="value" fill="#4DB6AC" radius={[4, 4, 0, 0]} />
          </BarChart>
        );
    }
  })();

  const empty = isMulti ? multiRows.length === 0 : simpleRows.length === 0;

  return (
    <div className={cn("rounded-xl border border-border bg-card p-4", className)}>
      {title && (
        <Text variant="label" as="h2" className="mb-3">
          {title}
        </Text>
      )}
      <p className="text-[10px] text-muted mb-2">
        {xLabel} · {yLabel}
        {isMulti && series.length > 1 ? ` · ${series.length} séries` : ""}
      </p>
      {empty ? (
        <p className="text-sm text-muted py-12 text-center">Aucune donnée pour ce graphique.</p>
      ) : (
        <div style={{ width: "100%", height }}>
          <ResponsiveContainer>{chart}</ResponsiveContainer>
        </div>
      )}
    </div>
  );
}
