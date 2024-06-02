// SPDX-License-Identifier: MIT
pragma solidity 0.8.17;

contract Silo {
    mapping(address => string) public Providers;
    address[] private keys;

    function get() public view returns (address[] memory, string[] memory) {
        // return Providers[addr];

        string[] memory values = new string[](keys.length);
        for (uint i = 0; i < keys.length; i++) {
            values[i] = Providers[keys[i]];
        }
        return (keys, values);
    }

    function set(string memory addr) public {
        if (bytes(Providers[msg.sender]).length == 0) {
            keys.push(msg.sender);
        }

        Providers[msg.sender] = addr;
    }

    function remove() public {
        delete Providers[msg.sender];

        for (uint i = 0; i < keys.length; i++) {
            if (keys[i] == msg.sender) {
                keys[i] = keys[keys.length - 1];
                keys.pop();
                break;
            }
        }
    }
}
