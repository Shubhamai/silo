# -*- coding: utf-8 -*-
# Generated by the protocol buffer compiler.  DO NOT EDIT!
# source: silo.proto
# Protobuf Python Version: 5.26.1
"""Generated protocol buffer code."""
from google.protobuf import descriptor as _descriptor
from google.protobuf import descriptor_pool as _descriptor_pool
from google.protobuf import symbol_database as _symbol_database
from google.protobuf.internal import builder as _builder
# @@protoc_insertion_point(imports)

_sym_db = _symbol_database.Default()




DESCRIPTOR = _descriptor_pool.Default().AddSerializedFile(b'\n\nsilo.proto\x12\x04silo\" \n\x11GetPackageRequest\x12\x0b\n\x03\x63id\x18\x01 \x01(\t\"4\n\x12GetPackageResponse\x12\x0e\n\x06output\x18\x02 \x01(\x0c\x12\x0e\n\x06\x65rrors\x18\x03 \x01(\t2I\n\x04Silo\x12\x41\n\nGetPackage\x12\x17.silo.GetPackageRequest\x1a\x18.silo.GetPackageResponse\"\x00\x62\x06proto3')

_globals = globals()
_builder.BuildMessageAndEnumDescriptors(DESCRIPTOR, _globals)
_builder.BuildTopDescriptorsAndMessages(DESCRIPTOR, 'silo_pb2', _globals)
if not _descriptor._USE_C_DESCRIPTORS:
  DESCRIPTOR._loaded_options = None
  _globals['_GETPACKAGEREQUEST']._serialized_start=20
  _globals['_GETPACKAGEREQUEST']._serialized_end=52
  _globals['_GETPACKAGERESPONSE']._serialized_start=54
  _globals['_GETPACKAGERESPONSE']._serialized_end=106
  _globals['_SILO']._serialized_start=108
  _globals['_SILO']._serialized_end=181
# @@protoc_insertion_point(module_scope)
