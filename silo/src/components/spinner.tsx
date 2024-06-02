import { Loader } from "lucide-react";

export const Spinner = () => {
  return (
    <div className="flex items-center justify-center w-fit">
      {/* <div className="animate-spin rounded-full h-32 w-32 border-t-2 border-b-2 border-foreground/50"> */}
      <Loader className="animate-spin size-5" />
      {/* </div> */}
    </div>
  );
};
