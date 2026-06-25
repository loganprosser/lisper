import React from "react";

const LisperTextLogo = ({
  width,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => (
  <svg
    width={width}
    height={height}
    className={className}
    viewBox="0 0 360 100"
    xmlns="http://www.w3.org/2000/svg"
  >
    <text
      x="0"
      y="74"
      fontFamily="system-ui, -apple-system, sans-serif"
      fontSize="84"
      fontWeight="700"
      className="logo-primary"
      fill="currentColor"
    >
      {"lisper"}
    </text>
  </svg>
);

export default LisperTextLogo;
