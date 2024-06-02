'use client';

/* eslint-disable @typescript-eslint/no-explicit-any */
import { ConnectButton } from "@rainbow-me/rainbowkit";

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

import { BecomeComputeProvider } from "./modules/become-compute-provider";
import { ComputeYourFunction } from "./modules/compute-your-function";
import { Footer } from "./modules/footer";

const Home = () => {
  return (
    <div className="w-full h-full min-h-screen py-12 mx-auto max-w-3xl">
      <div className="mb-16 w-full flex items-center justify-between">
        <div>
          <h1 className="text-3xl"># Silo</h1>
          <p className="text-foreground/85">Run distributed compute in seconds.</p>
        </div>

        <div className="mt-6 outline outline-1 outline-[#3898ff] outline-offset-4 w-fit rounded-lg">
          <ConnectButton />
        </div>
      </div>

      <Tabs defaultValue="account">
        <TabsList className="border mb-12 border-foreground/50 outline outline-8 outline-foreground outline-offset-4 w-fit rounded-lg">
          <TabsTrigger value="account" className="w-56">
            Compute your Function
          </TabsTrigger>
          <TabsTrigger value="password" className="w-56">
            Become a Compute Provider
          </TabsTrigger>
        </TabsList>

        <TabsContent className="flex flex-col gap-4" value="account">
          <ComputeYourFunction />
        </TabsContent>

        <TabsContent className="flex flex-col gap-4" value="password">
          <BecomeComputeProvider />
        </TabsContent>
      </Tabs>

      <Footer />
    </div>
  );
};

export default Home;
