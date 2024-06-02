export const Footer = () => {
  return (
    <div className="absolute bottom-6">
      <div className="w-60">
        <div className="flex items-center justify-between">
          <p className="uppercase text-foreground/70">github:</p>
          <a
            href="https://github.com/shubhamai/silo"
            target="_blank"
            className="text-blue-500 underline"
          >
            github.com/shubhamai/silo
          </a>
        </div>

        {/* <div className="flex items-center justify-between">
          <p className="uppercase text-foreground/70">slides:</p>
          <a
            href="https://github.com/shubhamai/silo"
            target="_blank"
            className="text-blue-500 underline"
          >
            github.com/shubhamai/silo
          </a>
        </div> */}
      </div>

      <div className="flex flex-col mt-6">
        <p className="uppercase text-foreground/70">BUILD WITH:</p>
        <p className="uppercase text-foreground">Filecoin, IPFS, Filcoin Virtual Machine, Lit Protocol</p>
      </div>

      <div className="flex flex-col mt-6">
        <p className="uppercase text-foreground/70">BUILD BY:</p>

        <div className="flex items-center gap-2">
          <a
            href="https://github.com/shubhamai"
            target="_blank"
            className="text-blue-500 underline"
          >
            github.com/shubhamai
          </a>

          <p className="uppercase text-foreground">&</p>

          <a
            href="https://github.com/denosaurabh"
            target="_blank"
            className="text-blue-500 underline"
          >
            github.com/denosaurabh
          </a>
        </div>
      </div>
    </div>
  );
};
