import React from "react";
import { useTranslation } from "react-i18next";
import mark from "../../assets/notype-mark.svg";

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
      <image href={mark} x="0" y="10" width="60" height="60" />
      <text
        x="150"
        y="55"
        textAnchor="middle"
        fill="currentColor"
        fontFamily="system-ui, sans-serif"
        fontSize="48"
        fontWeight="700"
        letterSpacing="-3"
      >
        {t("app.name")}
      </text>
    </svg>
  );
};

export default HandyTextLogo;
