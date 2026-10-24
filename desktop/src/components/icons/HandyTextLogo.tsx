import React from "react";
import { useTranslation } from "react-i18next";

const HandyTextLogo = ({
  width = 200,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => {
  const { t } = useTranslation();
  return (
    <svg
      width={width}
      height={height ?? width / 3}
      viewBox="0 0 240 80"
      className={className}
      role="img"
      aria-label={t("app.name")}
    >
      <text
        x="120"
        y="57"
        textAnchor="middle"
        fill="currentColor"
        fontFamily="system-ui, sans-serif"
        fontSize="60"
        fontWeight="700"
        letterSpacing="-3"
      >
        {t("app.name")}
      </text>
    </svg>
  );
};

export default HandyTextLogo;
