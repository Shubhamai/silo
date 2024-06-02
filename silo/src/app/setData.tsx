import * as React from 'react'
import {
  type BaseError,
  useWaitForTransactionReceipt,
  useWriteContract
} from 'wagmi'
import { abi } from '../../../fevm-hardhat-kit/deployments/calibrationnet/Silo.json'

// wagmi example
export function IncData() {
  const {
    data: hash,
    error,
    isPending,
    writeContract
  } = useWriteContract()

  async function submit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault()
    const formData = new FormData(e.target as HTMLFormElement)
    const tokenId = formData.get('tokenId') as string
    writeContract({
      address: '0x9591b53c11caB0F9E1776423a622b4c5529D45Dd',
      abi,
      functionName: 'set',
      // args: [BigInt(tokenId)],
      // string args
      args: ["localhost:50051"],
    })
  }

  const { isLoading: isConfirming, isSuccess: isConfirmed } =
    useWaitForTransactionReceipt({
      hash,
    })

  return (
    <form onSubmit={submit}>
      {/* <input name="address" placeholder="0xA0Cf…251e" required />
      <input name="value" placeholder="0.05" required /> */}
      <button
        disabled={isPending}
        type="submit"
      >
        {isPending ? 'Confirming...' : 'Add Provider'}
      </button>
      {hash && <div>Transaction Hash: {hash}</div>}
      {isConfirming && <div>Waiting for confirmation...</div>}
      {isConfirmed && <div>Transaction confirmed.</div>}
      {error && (
        <div>Error: {(error as BaseError).shortMessage || error.message}</div>
      )}
    </form>
  )
}