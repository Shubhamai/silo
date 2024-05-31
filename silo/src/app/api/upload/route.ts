import lighthouse from '@lighthouse-web3/sdk'
import LitJsSdk from 'lit-js-sdk';

const apiKey = process.env.LIGHTHOUSE_API_KEY

const client = new LitJsSdk.LitNodeClient({ alertWhenUnauthorized: false })
client.connect().then(() => {});

export async function POST(request: Request) {
    const data = await request.text()
  
    const { encryptedString: encryptedData, symmetricKey } = await LitJsSdk.encryptString(data);
    const encryptedDataInString = await LitJsSdk.blobToBase64String(encryptedData);
    const symmetricKeyInString = LitJsSdk.uint8arrayToString(symmetricKey, "base16");

    const response = await lighthouse.uploadText(encryptedDataInString, apiKey || '')
    
    return Response.json({
        hash: response['data']['Hash'],
        key: symmetricKeyInString
    })
}

export async function PATCH(request: Request) {
    const requestData = await request.json()
    const decryptedData = await LitJsSdk.decryptString(await LitJsSdk.base64StringToBlob(requestData['data']), 
    LitJsSdk.uint8arrayFromString(requestData['key'], "base16"));

    return Response.json(JSON.parse(decryptedData))

}
