import { formatSpeed } from "../utils/format";

interface SpeedSparklineProps {
  historyBps: number[];
  active: boolean;
}

export function SpeedSparkline({ historyBps, active }: SpeedSparklineProps) {
  const width = 316;
  const height = 38;
  const padX = 2;
  const padY = 4;

  const samples = historyBps.length >= 2 ? historyBps : [0, 0];
  const peak = Math.max(...samples, 16 * 1024); // Floor scale at 16 KB/s so tiny blips look gentle
  const hasTraffic = samples.some((v) => v >= 1024);

  const stepX = (width - padX * 2) / (samples.length - 1);
  const points = samples.map((val, idx) => {
    const x = padX + idx * stepX;
    const ratio = Math.min(1, val / peak);
    const y = height - padY - ratio * (height - padY * 2);
    return { x, y };
  });

  const linePath = points
    .map((pt, i) => `${i === 0 ? "M" : "L"}${pt.x.toFixed(1)},${pt.y.toFixed(1)}`)
    .join(" ");

  const areaPath = `${linePath} L${points[points.length - 1].x.toFixed(1)},${height} L${points[0].x.toFixed(1)},${height} Z`;

  return (
    <div className={`sparkline-box ${active && hasTraffic ? "sparkline-active" : ""}`}>
      <div className="sparkline-header">
        <span className="sparkline-label">Throughput</span>
        <span className="sparkline-peak tabular">
          {hasTraffic ? `Peak ${formatSpeed(peak)}` : "Idle"}
        </span>
      </div>
      <svg
        viewBox={`0 0 ${width} ${height}`}
        className="sparkline-svg"
        role="img"
        aria-label={hasTraffic ? `Transfer speed sparkline, peak ${formatSpeed(peak)}` : "Idle transfer graph"}
      >
        <defs>
          <linearGradient id="hudSparkFill" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--accent-sync)" stopOpacity="0.28" />
            <stop offset="100%" stopColor="var(--accent-sync)" stopOpacity="0.01" />
          </linearGradient>
        </defs>
        <line
          x1={padX}
          y1={height - padY}
          x2={width - padX}
          y2={height - padY}
          stroke="var(--border-subtle)"
          strokeWidth="1"
          strokeDasharray="2 3"
        />
        <path d={areaPath} fill="url(#hudSparkFill)" />
        <path
          d={linePath}
          fill="none"
          stroke={hasTraffic ? "var(--accent-sync)" : "var(--text-muted)"}
          strokeOpacity={hasTraffic ? 1 : 0.35}
          strokeWidth="1.75"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    </div>
  );
}
