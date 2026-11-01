import mark from "../../assets/notype-mark.svg";

const HandyHand = ({
  width = 126,
  height = width,
}: {
  width?: number | string;
  height?: number | string;
}) => <img src={mark} width={width} height={height} alt="" />;

export default HandyHand;
