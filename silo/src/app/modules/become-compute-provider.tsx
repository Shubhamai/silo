import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import { z } from "zod";

import SiloABI from "../../../../fevm-hardhat-kit/deployments/calibrationnet/Silo.json";
import { Spinner } from "../../components/spinner";

import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";

import CopyButton from "../../components/copy-button";
import { Button } from "@/components/ui/button";
import { useAccount, useWriteContract } from "wagmi";

const FormSchema = z.object({
  address: z.string().min(2, {
    message: "Username must be at least 2 characters.",
  }),
  ip: z.string().min(2, {
    message: "IP must be at least 2 characters.",
  }),
});

export const BecomeComputeProvider = () => {
  const { address } = useAccount();

  const form = useForm<z.infer<typeof FormSchema>>({
    resolver: zodResolver(FormSchema),
    defaultValues: {
      address,
      ip: "",
    },
  });

  const { writeContract, isPending, isSuccess } = useWriteContract();

  const onSubmit = (data: z.infer<typeof FormSchema>) => {
    console.log(data);

    writeContract({
      address: SiloABI.address as unknown as `0x${string}`,
      abi: SiloABI.abi,
      functionName: "set",
      // args: [BigInt(data.address), BigInt(data.ip)],
      args: [data.ip],
    });
  };

  return (
    <>
      {/* <p className="text-foreground/80">
        run the following command in your terminal to install the silo
      </p>

      <div className="border-foreground/30 border flex items-center justify-between px-3 py-1 pr-1">
        <p className="text-emerald-600">
          curl sh -c "$(curl -fsSL https://silo.sh)"
        </p>

        <CopyButton textToCopy={`curl sh -c "$(curl -fsSL https://silo.sh)"`} />
      </div> */}

      <h2 className="mt-8 text-2xl">## Add yourself to Providers List</h2>

      <Form {...form}>
        <form
          onSubmit={form.handleSubmit(onSubmit)}
          className="flex flex-col gap-1"
        >
          <FormField
            control={form.control}
            name="address"
            render={({ field }) => (
              <FormItem>
                <div className="w-full flex items-center justify-between">
                  <FormLabel>Address</FormLabel>
                  <FormControl>
                    <Input className="w-96" placeholder="0x" {...field} />
                  </FormControl>
                </div>

                <FormDescription className="text-right">
                  This is your filecoin address.
                </FormDescription>
                <FormMessage className="text-right" />
              </FormItem>
            )}
          />

          <FormField
            control={form.control}
            name="ip"
            render={({ field }) => (
              <FormItem>
                <div className="w-full flex items-center justify-between">
                  <FormLabel>IP</FormLabel>
                  <FormControl>
                    <Input
                      className="w-96"
                      placeholder="http://123.45.67.8"
                      {...field}
                    />
                  </FormControl>
                </div>

                <FormDescription className="text-right">
                  This is your public IP address.
                </FormDescription>
                <FormMessage className="text-right" />
              </FormItem>
            )}
          />

          <Button
            type="submit"
            className="ml-auto bg-emerald-700 text-foreground/90 text-lg hover:bg-emerald-600 mt-6 w-40"
            disabled={isPending || isSuccess}
          >
            {isPending ? <Spinner /> : isSuccess ? "Submitted" : "Submit"}
          </Button>
        </form>
      </Form>
    </>
  );
};
