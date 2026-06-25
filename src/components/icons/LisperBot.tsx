const LisperBot = ({
  width,
  height,
}: {
  width?: number | string;
  height?: number | string;
}) => (
  <svg
    width={width || 128}
    height={height || 128}
    viewBox="0 0 512 512"
    className="fill-text stroke-text"
    xmlns="http://www.w3.org/2000/svg"
  >
    <line
      x1="256"
      y1="96"
      x2="256"
      y2="150"
      strokeWidth="14"
      strokeLinecap="round"
    />
    <circle cx="256" cy="90" r="18" />
    <rect
      x="136"
      y="150"
      width="240"
      height="200"
      rx="44"
      className="fill-logo-primary"
    />
    <circle cx="206" cy="240" r="26" />
    <circle cx="306" cy="240" r="26" />
    <rect x="196" y="296" width="120" height="18" rx="9" />
  </svg>
);

export default LisperBot;
