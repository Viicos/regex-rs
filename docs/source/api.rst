API reference
=============

.. module:: rustregex

.. autofunction:: escape

.. data:: REGEX_VERSION
   :type: str

   The version of the Rust ``regex`` crate this module was built against.

Regular expressions
-------------------

.. autoclass:: Regex
   :members:

.. autoclass:: Match
   :members:
   :special-members: __len__

.. autoclass:: Captures
   :members:
   :special-members: __getitem__, __len__, __iter__

Regex sets
----------

.. autoclass:: RegexSet
   :members:
   :special-members: __len__

.. autoclass:: SetMatches
   :members:

Iterators
---------

.. autoclass:: Matches

.. autoclass:: CaptureMatches

.. autoclass:: Split

.. autoclass:: SplitN

.. autoclass:: SubCaptureMatches

.. autoclass:: SetMatchesIter

Errors
------

.. autoexception:: RegexError

.. autoexception:: PatternSyntaxError

.. autoexception:: CompiledTooBigError
   :members: limit
