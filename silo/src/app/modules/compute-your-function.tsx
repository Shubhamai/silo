import { useReadContract } from "wagmi";

import SiloABI from "../../../../fevm-hardhat-kit/deployments/calibrationnet/Silo.json";
import { Spinner } from "../../components/spinner";
import { useMemo } from "react";
import CopyButton from "../../components/copy-button";

export const ComputeYourFunction = () => {
  const { data, isLoading, isError } = useReadContract({
    abi: SiloABI.abi,
    address: SiloABI.address as unknown as `0x${string}`,
    functionName: "get",
  });

  const parsedData = useMemo(() => {
    if (!data || !data.length) return null;

    try {
      let res = [];
      for (let i = 0; i < data[0].length; i++) {
        res.push({
          address: data[0][i],
          ip: data[1][i],
        });
      }

      return res;
    } catch (e) {
      return [];
    }
  }, [data, isLoading]);

  return (
    <>
      <h2 className="text-2xl text-foreground/80">Active Providers</h2>

      {isLoading ? (
        <Spinner />
      ) : isError ? (
        <p className="text-sm text-red-500">Couldn't fetch providers.</p>
      ) : !parsedData.length ? (
        <p className="text-sm text-orange-500">No Active Providers.</p>
      ) : (
        <div className="flex flex-col gap-3">
          {parsedData.map((provider, i) => (
            <div
              key={i}
              className="border-foreground/30 border flex items-center justify-between px-3 py-1 pr-1"
            >
              <div>
                <p className="text-emerald-600">
                  Provider: &nbsp;&nbsp; {provider.address}
                </p>
                <p>
                  IP: &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;{" "}
                  {provider.ip}
                </p>
              </div>
              <CopyButton textToCopy={`${provider.ip}`} />
            </div>
          ))}
        </div>
      )}
    </>
  );
};
