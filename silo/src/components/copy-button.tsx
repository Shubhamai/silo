import { useState } from "react";
import { Button } from "./ui/button";

// import {
//   Tooltip,
//   TooltipContent,
//   TooltipProvider,
//   TooltipTrigger,
// } from "@/components/ui/tooltip";

import { CheckIcon, CopyIcon } from "lucide-react";

const CopyButton = ({ textToCopy }: { textToCopy: string }) => {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(textToCopy).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };

  return (
    // <TooltipProvider>
    //   <Tooltip>
    //     <TooltipTrigger asChild>
    <Button
      className="outline-none outline-0"
      size="icon"
      variant="ghost"
      onClick={handleCopy}
    >
      {copied ? (
        <CheckIcon className="size-4" />
      ) : (
        <CopyIcon className="size-4" />
      )}
    </Button>

    // </TooltipTrigger>
    // <TooltipContent>Copy</TooltipContent>
    //   </Tooltip>
    // </TooltipProvider>
  );
};

export default CopyButton;
