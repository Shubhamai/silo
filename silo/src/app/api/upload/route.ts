import lighthouse from '@lighthouse-web3/sdk'

const apiKey = process.env.LIGHTHOUSE_API_KEY

export async function POST(request: Request) {
    const data = await request.text()
    // console.log(data)
    const response = await lighthouse.uploadText(data, apiKey || '')
    return Response.json(response)
    // return Response.json(data)
}